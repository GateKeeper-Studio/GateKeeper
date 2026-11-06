use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::shared::token_policy::WEBAUTHN_CHALLENGE_TTL_MINUTES;
use crate::{
    application::{
        ports::{unit_of_work::UnitOfWorkFactory, webauthn::WebauthnService},
        use_cases::mfa::start_webauthn_login::{command::Command, response::Response},
    },
    domain::{
        entities::{
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_method::MfaMethod,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::sha256_hex,
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    webauthn: Arc<dyn WebauthnService>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        webauthn: Arc<dyn WebauthnService>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            webauthn,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();
        let invalid = || DomainError::AuthenticationError("invalid or expired session".to_string());

        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active() && c.allows_mfa_method(&MfaMethod::Webauthn))
            .ok_or_else(invalid)?;

        let session = uow
            .auth_session_repository()
            .find_by_token_hash(client.tenant_id(), &sha256_hex(&command.session_code))
            .await?
            .ok_or_else(invalid)?;

        if session.is_expired(now)
            || !session.is_authenticated()
            || !session.is_mfa_pending()
            || !session.accepts_mfa_method(&MfaMethod::Webauthn)
            || session.pending_client_id() != client.id()
        {
            return Err(invalid());
        }

        let user_id = session.user_id().ok_or_else(invalid)?;
        let enrollments = uow
            .mfa_enrollment_repository()
            .find_confirmed_by_user_id(user_id)
            .await?;
        let passkeys: Vec<_> = enrollments
            .iter()
            .filter(|e| *e.method() == MfaMethod::Webauthn)
            .filter_map(|e| e.passkey_json().cloned())
            .collect();
        if passkeys.is_empty() {
            return Err(invalid());
        }

        // Replace only THIS session's live ceremony. A passkey session is opened from
        // an email alone, so a per-user invalidation would let anyone who knows the
        // email cancel the real user's sign-in running in another session.
        uow.mfa_challenge_repository()
            .invalidate_active_by_session(
                session.id(),
                &MfaChallengePurpose::Login,
                &MfaMethod::Webauthn,
            )
            .await?;

        let ceremony = self.webauthn.start_authentication(&passkeys)?;
        let challenge = MfaChallenge::new_webauthn(
            client.tenant_id(),
            user_id,
            Some(session.id()),
            MfaChallengePurpose::Login,
            ceremony.state,
            now + Duration::minutes(WEBAUTHN_CHALLENGE_TTL_MINUTES),
        );
        let mfa_id = challenge.id().to_string();
        uow.mfa_challenge_repository().add(&challenge).await?;
        uow.commit().await?;

        Ok(Response {
            mfa_id,
            options: ceremony.options,
        })
    }
}
