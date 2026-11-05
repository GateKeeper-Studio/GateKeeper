use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::application::shared::token_policy::RESET_TOKEN_TTL_MINUTES;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        services::templated_email_sender::TemplatedEmailSender,
        use_cases::authentication::forgot_password::command::Command,
    },
    domain::{entities::password_reset_token::PasswordResetToken, errors::DomainError},
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// 30 minutes is a comfortable window to receive the email and click the link,
// while keeping a forgotten or leaked link short-lived.

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    templated_email: Arc<TemplatedEmailSender>,
    // Base URL of the hosted login UI (dashboard) the reset link points at.
    auth_ui_base_url: String,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        templated_email: Arc<TemplatedEmailSender>,
        auth_ui_base_url: String,
    ) -> Self {
        Self {
            unit_of_work_factory,
            templated_email,
            auth_ui_base_url,
        }
    }

    // Always returns Ok(()) regardless of whether the account exists or the
    // feature is enabled — the caller must not be able to enumerate accounts via
    // this endpoint.
    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // 1. Resolve the application by its public client_id and derive the tenant.
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        let tenant_id = client.tenant_id();

        // 1b. There is no password to reset on an external-login-only client. Unlike
        //     the checks below, this one is not an enumeration risk: the flag is part
        //     of the client's PUBLIC auth data, so refusing outright is safe and tells
        //     the caller something it could already read.
        if !client.allows_password_login() {
            return Err(DomainError::AccessDenied(
                "this application signs in with an external provider only".to_string(),
            ));
        }

        let tenant = uow
            .tenant_repository()
            .find_by_id(tenant_id)
            .await?
            .filter(|t| t.is_active())
            .ok_or_else(|| DomainError::NotFound(format!("tenant {tenant_id} not found")))?;

        // 2. Self-service password reset must be enabled for the tenant. Silently
        //    succeed if not — the feature state is not an enumerable secret.
        if !tenant.can_self_forgot_password() {
            return Ok(());
        }

        // 3. If the user doesn't exist or is inactive, do nothing but return Ok so
        //    callers cannot learn anything about account state.
        let user = match uow
            .user_repository()
            .find_by_email(tenant_id, &command.email)
            .await?
        {
            Some(u) if u.is_active() => u,
            _ => return Ok(()),
        };

        // 4. Single live token: burn any older still-active reset links first.
        uow.password_reset_token_repository()
            .invalidate_active_by_user_id(user.id())
            .await?;

        // 5. Mint the opaque token — the raw value is emailed, only its SHA-256 hash
        //    is persisted (looked up later by the token's row id).
        let raw_token = generate_opaque_token();
        let token = PasswordResetToken::new(
            tenant_id,
            user.id(),
            sha256_hex(&raw_token),
            Utc::now() + Duration::minutes(RESET_TOKEN_TTL_MINUTES),
        );
        let reset_id = token.id();
        uow.password_reset_token_repository().add(&token).await?;

        // Resolve which SMTP provider sends this client's mail (its own, else the
        // tenant default) while still inside the transaction.
        let email_provider = match client.email_provider_id() {
            Some(provider_id) => {
                uow.email_provider_repository()
                    .find_by_id(tenant_id, provider_id)
                    .await?
            }
            None => {
                uow.email_provider_repository()
                    .find_default_by_tenant_id(tenant_id)
                    .await?
            }
        }
        .filter(|p| p.is_active());

        uow.commit().await?;

        // 6. Send the link AFTER commit, so a delivery hiccup never leaves a token
        //    the user can't see. The dashboard reset page reads `?token=..&id=..`.
        let link = build_reset_link(
            &self.auth_ui_base_url,
            &command.client_id,
            &raw_token,
            reset_id,
        );
        self.templated_email
            .send_password_reset_link(
                tenant_id,
                email_provider.as_ref(),
                &command.email,
                &link,
                client.name(),
                &format!("{RESET_TOKEN_TTL_MINUTES} minutes"),
            )
            .await?;

        Ok(())
    }
}

// {base}/auth/{client_id}/change-password?token={raw}&id={reset_id}
//
// The raw token (URL-safe base64) and the UUID id need no extra encoding.
fn build_reset_link(base_url: &str, client_id: &str, raw_token: &str, reset_id: Uuid) -> String {
    format!(
        "{}/auth/{}/change-password?token={}&id={}",
        base_url.trim_end_matches('/'),
        client_id,
        raw_token,
        reset_id,
    )
}
