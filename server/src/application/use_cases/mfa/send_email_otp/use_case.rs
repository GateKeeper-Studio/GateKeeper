use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::shared::token_policy::MFA_CODE_TTL_MINUTES;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        services::templated_email_sender::TemplatedEmailSender,
        use_cases::mfa::send_email_otp::command::Command,
    },
    domain::{
        entities::{
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_method::MfaMethod,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_numeric_code, sha256_hex},
};

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

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();
        let invalid = || DomainError::AuthenticationError("invalid or expired session".to_string());

        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active() && c.allows_mfa_method(&MfaMethod::Email))
            .ok_or_else(invalid)?;

        let mut session = uow
            .auth_session_repository()
            .find_by_token_hash(client.tenant_id(), &sha256_hex(&command.session_code))
            .await?
            .ok_or_else(invalid)?;

        if session.is_expired(now)
            || !session.is_authenticated()
            || !session.is_mfa_pending()
            || session.pending_client_id() != client.id()
        {
            return Err(invalid());
        }

        let user_id = session.user_id().ok_or_else(invalid)?;
        let user = uow
            .user_repository()
            .find_by_id(user_id)
            .await?
            .filter(|u| {
                u.tenant_id() == client.tenant_id() && u.is_active() && u.is_email_verified()
            })
            .ok_or_else(invalid)?;

        uow.mfa_challenge_repository()
            .invalidate_active_by_user(user_id, &MfaChallengePurpose::Login, &MfaMethod::Email)
            .await?;

        let raw_code = generate_numeric_code();
        let challenge = MfaChallenge::new_email_otp(
            client.tenant_id(),
            user_id,
            Some(session.id()),
            MfaChallengePurpose::Login,
            sha256_hex(&raw_code),
            now + Duration::minutes(MFA_CODE_TTL_MINUTES),
        );
        uow.mfa_challenge_repository().add(&challenge).await?;

        let email_provider = match client.email_provider_id() {
            Some(provider_id) => {
                uow.email_provider_repository()
                    .find_by_id(client.tenant_id(), provider_id)
                    .await?
            }
            None => {
                uow.email_provider_repository()
                    .find_default_by_tenant_id(client.tenant_id())
                    .await?
            }
        }
        .filter(|p| p.is_active());

        session.require_mfa();
        uow.auth_session_repository().update(&session).await?;
        uow.commit().await?;

        self.templated_email
            .send_mfa_code(
                client.tenant_id(),
                email_provider.as_ref(),
                user.email(),
                &raw_code,
                client.name(),
                &format!("{MFA_CODE_TTL_MINUTES} minutes"),
            )
            .await?;
        Ok(())
    }
}
