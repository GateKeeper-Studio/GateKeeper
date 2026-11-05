use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        services::templated_email_sender::TemplatedEmailSender,
        shared::custom_field_validation::validate_custom_field_values,
        use_cases::{
            authentication::register::{command::Command, response::Response},
            webhook::user_event_payload::user_event,
        },
    },
    domain::{
        entities::{
            domain_event_type::{DomainEventType, UserEventSource},
            email_verification_token::EmailVerificationToken,
            user::User,
            user_credentials::UserCredentials,
            user_profile::UserProfile,
        },
        errors::DomainError,
    },
    utils::{
        oauth_crypto::{generate_numeric_code, sha256_hex},
        password_hasher::hash_password,
    },
};

// 15 minutes is the standard window for a human-typed numeric code — long enough
// to find the email, short enough to limit the brute-force window.
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

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // 1. Resolve the application by its globally-unique public client_id and
        //    derive the tenant from it — the caller never supplies a tenant id.
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        let tenant_id = client.tenant_id();

        // 1b. Users of an external-login-only client never pick a password — they are
        //     provisioned by the social callback instead.
        if !client.allows_password_login() {
            return Err(DomainError::AccessDenied(
                "this application signs up with an external provider only".to_string(),
            ));
        }

        // 2. Self-registration must be enabled for the owning tenant.
        let tenant = uow
            .tenant_repository()
            .find_by_id(tenant_id)
            .await?
            .filter(|t| t.is_active())
            .ok_or_else(|| DomainError::TenantNotFound(format!("tenant {tenant_id} not found")))?;

        if !tenant.can_self_register() {
            return Err(DomainError::AccessDenied(
                "self-registration is disabled for this tenant".to_string(),
            ));
        }

        // 3. Reject duplicates. NOTE: this reveals whether an email is registered
        //    (an account-enumeration tradeoff) in exchange for clear UX.
        if uow
            .user_repository()
            .is_user_with_email_exists(tenant_id, &command.email)
            .await?
        {
            return Err(DomainError::UserAlreadyExists(format!(
                "User with email {} already exists",
                command.email
            )));
        }

        // 4. Argon2 is intentionally CPU/memory heavy; run it off the async worker.
        let password = command.password.clone();
        let secret = tenant.password_hash_secret().to_string();
        let hashed_password =
            tokio::task::spawn_blocking(move || hash_password(&password, &secret))
                .await
                .map_err(|_| {
                    DomainError::PasswordHashingError("Failed to hash password".to_string())
                })?
                .map_err(|_| {
                    DomainError::PasswordHashingError("Failed to hash password".to_string())
                })?;

        // 5. Build the (unverified) user with credentials + profile.
        let custom_field_definitions = uow
            .custom_field_definition_repository()
            .find_active_by_tenant_id(tenant_id)
            .await?;
        let custom_field_values = validate_custom_field_values(
            &custom_field_definitions,
            command.custom_field_values.clone(),
        )?;

        let mut user = User::new(tenant_id, command.email.clone(), None, None);
        user.set_credentials(UserCredentials::new(
            user.id(),
            hashed_password,
            "argon2".to_string(),
            false,
        ));
        let mut profile = UserProfile::new(
            user.id(),
            command.display_name.clone(),
            command.first_name.clone(),
            command.last_name.clone(),
        );
        profile.set_custom_field_values(custom_field_values);
        user.set_profile(profile);
        uow.user_repository().add(&user).await?;

        // Announce the new user to this tenant's subscribers. Enqueued INSIDE the
        // transaction, so the event exists if and only if the user does — no window
        // where a crash leaves a user nobody downstream ever hears about.
        uow.outbox_event_repository()
            .enqueue(&user_event(
                DomainEventType::UserCreated,
                &user,
                Some(UserEventSource::SelfRegistration),
            ))
            .await?;

        // 6. Mint the 6-digit verification code — the raw value is emailed, only its
        //    SHA-256 hash is persisted.
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

        // 7. Send the confirmation code AFTER commit, so a delivery hiccup can never
        //    leave a half-written account behind.
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

        Ok(Response {
            message: "Registration successful. Check your email for the confirmation code."
                .to_string(),
        })
    }
}
