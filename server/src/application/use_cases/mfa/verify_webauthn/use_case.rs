use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::{unit_of_work::UnitOfWorkFactory, webauthn::WebauthnService},
        use_cases::mfa::verify_webauthn::{command::Command, response::Response},
    },
    domain::{
        entities::{mfa_challenge::MfaChallengePurpose, mfa_method::MfaMethod},
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
        let invalid = || DomainError::AuthenticationError("invalid MFA challenge".to_string());

        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active() && c.allows_mfa_method(&MfaMethod::Webauthn))
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
        let mut challenge = uow
            .mfa_challenge_repository()
            .find_by_id(client.tenant_id(), command.mfa_id)
            .await?
            .filter(|c| {
                c.user_id() == user_id
                    && c.auth_session_id() == Some(session.id())
                    && *c.purpose() == MfaChallengePurpose::Login
                    && *c.method() == MfaMethod::Webauthn
                    && c.is_redeemable(now)
            })
            .ok_or_else(invalid)?;

        let mut enrollments = uow
            .mfa_enrollment_repository()
            .find_confirmed_by_user_id(user_id)
            .await?;
        let passkeys: Vec<_> = enrollments
            .iter()
            .filter(|e| *e.method() == MfaMethod::Webauthn)
            .filter_map(|e| e.passkey_json().cloned())
            .collect();
        let state = challenge.state_json().ok_or_else(invalid)?;
        let authenticated =
            self.webauthn
                .finish_authentication(&command.assertion_data, state, &passkeys)?;
        let enrollment = enrollments
            .iter_mut()
            .find(|e| e.credential_id() == Some(authenticated.credential_id.as_str()))
            .ok_or_else(invalid)?;

        if let Some(updated) = authenticated.updated_passkey {
            enrollment.record_webauthn_use(updated, now);
            uow.mfa_enrollment_repository().update(enrollment).await?;
        }

        challenge.consume(now);
        session.verify_mfa(&MfaMethod::Webauthn, now);
        uow.mfa_challenge_repository().update(&challenge).await?;
        uow.auth_session_repository().update(&session).await?;
        uow.commit().await?;

        Ok(Response {
            session_code: command.session_code,
        })
    }
}
