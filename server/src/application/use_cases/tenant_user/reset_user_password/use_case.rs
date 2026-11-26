use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::ports::password_hasher::PasswordHasher;
use crate::application::shared::backchannel_logout::{EndedSessions, announce_ended_sessions};
use crate::application::shared::token_policy::TEMPORARY_PASSWORD_TTL_HOURS;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        services::templated_email_sender::TemplatedEmailSender,
        shared::actor::Actor,
        use_cases::tenant_user::reset_user_password::{command::Command, response::Response},
    },
    domain::{
        entities::{
            audit_action::USER_PASSWORD_ADMIN_RESET,
            audit_event::{ActorType, AuditEvent, Outcome},
            email_template::EmailTemplateType,
            user_credentials::UserCredentials,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::generate_temporary_password,
};

// An admin resets a user's password. The server makes a temporary password, emails
// it to the user, and keeps only its Argon2 hash. The admin never sees it: the
// response, the audit row and the logs carry no trace of it.
//
// The temporary password signs in once, within TEMPORARY_PASSWORD_TTL_HOURS, and only
// to choose a new password (login burns it; authorize refuses until the change).
// The second factor is still asked for. Whatever the old password opened is closed:
// sessions, sign-ins in progress, trusted devices, reset links and change codes.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    password_hasher: Arc<dyn PasswordHasher>,
    templated_email: Arc<TemplatedEmailSender>,
    // Outside development an email with no SMTP provider is dropped, which would
    // leave the user with a password they never receive: refuse the reset instead.
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

        let mut user = uow
            .user_repository()
            .find_by_id_with_credentials(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("User with id {} not found", command.user_id))
            })?;
        if !user.is_active() {
            return Err(DomainError::Validation(
                "this account is deactivated".to_string(),
            ));
        }
        // Giving a password to an account that signs in with a passkey or a social
        // account would change how it signs in; that is not a reset.
        if user.credentials().is_none() {
            return Err(DomainError::Validation(
                "this account has no password to reset".to_string(),
            ));
        }

        // Checked before anything changes: a reset whose email cannot go out would
        // lock the user out.
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

        let temporary_password = generate_temporary_password();
        let hashed_password = self
            .password_hasher
            .hash(&temporary_password, tenant.password_hash_secret())
            .await?;
        let expires_at = Utc::now() + Duration::hours(TEMPORARY_PASSWORD_TTL_HOURS);
        user.update_credentials(UserCredentials::temporary(
            user.id(),
            hashed_password,
            "argon2".to_string(),
            expires_at,
        ));
        uow.user_repository().update(&user).await?;

        // Codes that would set a password without this one: an earlier temporary
        // password's change code, and a forgot-password link.
        uow.password_change_token_repository()
            .invalidate_active_by_user_id(user.id())
            .await?;
        uow.password_reset_token_repository()
            .invalidate_active_by_user_id(user.id())
            .await?;

        // Whoever got in with the old password is out: the apps are told
        // (back-channel logout), refresh tokens die, sign-ins in progress end, and
        // devices trusted under it are forgotten.
        announce_ended_sessions(&*uow, command.tenant_id, user.id(), EndedSessions::All).await?;
        uow.refresh_token_repository()
            .revoke_by_user(command.tenant_id, user.id())
            .await?;
        uow.auth_session_repository()
            .delete_by_user_id(user.id())
            .await?;
        uow.trusted_device_repository()
            .delete_all_for_user(command.tenant_id, user.id())
            .await?;

        uow.audit_event_repository()
            .record(
                &AuditEvent::new(
                    Some(command.tenant_id),
                    match actor {
                        Actor::System => ActorType::System,
                        _ => ActorType::Admin,
                    },
                    actor.id(),
                    USER_PASSWORD_ADMIN_RESET,
                    Outcome::Success,
                )
                .with_request_context(command.ip_address, command.user_agent)
                .with_resource("user", user.id())
                .with_metadata(serde_json::json!({
                    "user_id": user.id(),
                    "expires_at": expires_at,
                })),
            )
            .await?;

        uow.commit().await?;

        // Sent after the commit, like a reset link: a password that never committed
        // must not reach anyone. If delivery fails the admin sees the error and resets
        // again, which replaces this password.
        self.templated_email
            .send_temporary_password(
                command.tenant_id,
                EmailTemplateType::TemporaryPassword,
                email_provider.as_ref(),
                user.email(),
                &temporary_password,
                tenant.name(),
                &format!("{TEMPORARY_PASSWORD_TTL_HOURS} hours"),
            )
            .await?;

        Ok(Response {
            user_id: user.id(),
            email: user.email().to_string(),
            temporary_password_expires_at: expires_at,
        })
    }
}
