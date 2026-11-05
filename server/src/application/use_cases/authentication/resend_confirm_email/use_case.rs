use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        services::templated_email_sender::TemplatedEmailSender,
        use_cases::authentication::resend_confirm_email::command::Command,
    },
    domain::{entities::email_verification_token::EmailVerificationToken, errors::DomainError},
    utils::oauth_crypto::{generate_numeric_code, sha256_hex},
};

const VERIFICATION_CODE_TTL_MINUTES: i64 = 15;

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    templated_email: Arc<TemplatedEmailSender>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        templated_email: Arc<TemplatedEmailSender>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            templated_email,
        }
    }

    // Always returns Ok(()) regardless of whether the user or code exists — the
    // caller must not be able to enumerate accounts via this endpoint.
    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        let tenant_id = client.tenant_id();

        let tenant = uow
            .tenant_repository()
            .find_by_id(tenant_id)
            .await?
            .filter(|t| t.is_active())
            .ok_or_else(|| DomainError::TenantNotFound(format!("tenant {tenant_id} not found")))?;

        if !tenant.can_self_register() {
            // Silently succeed — self-registration state is not an enumerable secret.
            return Ok(());
        }

        // If the user doesn't exist or is already verified, do nothing but return Ok
        // so callers cannot learn anything about account state.
        let user = match uow
            .user_repository()
            .find_by_email(tenant_id, &command.email)
            .await?
        {
            Some(u) if u.is_active() && !u.is_email_verified() => u,
            _ => return Ok(()),
        };

        // Consume any existing active code so there is never more than one live code
        // per user at a time.
        if let Some(mut old) = uow
            .email_verification_token_repository()
            .find_active_by_user_id(user.id())
            .await?
        {
            old.consume(Utc::now());
            uow.email_verification_token_repository()
                .update(&old)
                .await?;
        }

        let raw_code = generate_numeric_code();
        let token = EmailVerificationToken::new(
            tenant_id,
            user.id(),
            sha256_hex(&raw_code),
            Utc::now() + Duration::minutes(VERIFICATION_CODE_TTL_MINUTES),
        );
        uow.email_verification_token_repository()
            .add(&token)
            .await?;

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

        self.templated_email
            .send_verification_code(
                tenant_id,
                email_provider.as_ref(),
                &command.email,
                &raw_code,
                client.name(),
                &format!("{VERIFICATION_CODE_TTL_MINUTES} minutes"),
            )
            .await?;

        Ok(())
    }
}
