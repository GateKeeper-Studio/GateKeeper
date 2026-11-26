use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::ports::password_hasher::PasswordHasher;
use crate::application::shared::actor::Actor;
use crate::application::shared::token_policy::NEW_ACCOUNT_PASSWORD_TTL_HOURS;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        services::templated_email_sender::TemplatedEmailSender,
        use_cases::{
            tenant_user::create_tenant_user::{command::Command, response::Response},
            webhook::user_event_payload::user_event,
        },
    },
    domain::{
        entities::{
            domain_event_type::{DomainEventType, UserEventSource},
            email_template::EmailTemplateType,
            user::User,
            user_credentials::UserCredentials,
            user_profile,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::generate_temporary_password,
};

// An admin creates a user. The server makes the first password and emails it to the
// user in a welcome email; the admin never sees it (not in the response, the audit
// row or the logs). Like an admin reset's, it signs in once, within
// NEW_ACCOUNT_PASSWORD_TTL_HOURS, and only to choose the user's own password.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    password_hasher: Arc<dyn PasswordHasher>,
    templated_email: Arc<TemplatedEmailSender>,
    // Outside development an email with no SMTP provider is dropped, which would
    // leave the account with a password nobody receives: refuse instead.
    require_email_provider: bool,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        password_hasher: Arc<dyn PasswordHasher>,
        templated_email: Arc<TemplatedEmailSender>,
        require_email_provider: bool,
    ) -> Self {
        Self {
            unit_of_work_factory,
            password_hasher,
            templated_email,
            require_email_provider,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        // A caller that still picks the password must learn it no longer does, rather
        // than believe the account opens with it.
        if command.temporary_password.is_some() {
            return Err(DomainError::Validation(
                "temporary_password is no longer accepted: the server emails the user a \
                 temporary password"
                    .to_string(),
            ));
        }

        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Tenant with id {} not found", command.tenant_id))
            })?;

        let is_user_already_exists = uow
            .user_repository()
            .is_user_with_email_exists(command.tenant_id, &command.email)
            .await?;

        if is_user_already_exists {
            return Err(DomainError::Conflict(format!(
                "User with email {} already exists",
                command.email
            )));
        }

        // Checked before anything is created: an account whose first password cannot
        // be delivered opens for nobody.
        let email_provider = uow
            .email_provider_repository()
            .find_default_by_tenant_id(command.tenant_id)
            .await?
            .filter(|p| p.is_active());
        if email_provider.is_none() && self.require_email_provider {
            return Err(DomainError::Validation(
                "this tenant has no active email provider, so the temporary password could \
                 not be delivered; configure one first"
                    .to_string(),
            ));
        }

        let mut user = User::new(command.tenant_id, command.email.clone(), None, None);

        if command.is_email_verified {
            user.verify_email();
        }

        // Argon2id runs behind the `PasswordHasher` port: bounded and off the async
        // workers.
        let temporary_password = generate_temporary_password();
        let hashed_password = self
            .password_hasher
            .hash(&temporary_password, tenant.password_hash_secret())
            .await?;
        let expires_at = Utc::now() + Duration::hours(NEW_ACCOUNT_PASSWORD_TTL_HOURS);

        // Single use, expiring, and the user must replace it at the first sign-in.
        let user_credentials = UserCredentials::temporary(
            user.id(),
            hashed_password,
            "argon2".to_string(),
            expires_at,
        );

        let user_profile = user_profile::UserProfile::new(
            user.id(),
            command.display_name,
            command.first_name.clone(),
            command.last_name.clone(),
        );

        user.set_credentials(user_credentials);
        user.set_profile(user_profile);

        uow.user_repository().add(&user).await?;

        // Same transaction as the insert: the event and the user commit together.
        uow.outbox_event_repository()
            .enqueue(&user_event(
                DomainEventType::UserCreated,
                &user,
                Some(UserEventSource::Admin),
            ))
            .await?;

        uow.commit().await?;

        // Sent after the commit: a password for an account that never committed must
        // not reach anyone. The account exists either way, so a failed delivery is
        // reported in the answer, not as an error; an admin reset sends a new one.
        let temporary_password_emailed = match self
            .templated_email
            .send_temporary_password(
                command.tenant_id,
                EmailTemplateType::AccountCreated,
                email_provider.as_ref(),
                &command.email,
                &temporary_password,
                tenant.name(),
                &format!("{NEW_ACCOUNT_PASSWORD_TTL_HOURS} hours"),
            )
            .await
        {
            Ok(()) => true,
            Err(err) => {
                tracing::warn!(
                    user_id = %user.id(),
                    "welcome email with the temporary password not delivered: {err}"
                );
                false
            }
        };

        Ok(Response {
            user_id: user.id(),
            display_name: user.profile().and_then(|p| p.display_name().cloned()),
            first_name: user
                .profile()
                .map(|p| p.first_name().to_string())
                .unwrap_or(command.first_name),
            last_name: user
                .profile()
                .map(|p| p.last_name().to_string())
                .unwrap_or(command.last_name),
            email: command.email,
            temporary_password_expires_at: expires_at,
            temporary_password_emailed,
        })
    }
}
