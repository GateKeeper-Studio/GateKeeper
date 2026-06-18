use std::sync::Arc;

use chrono::Utc;
use subtle::ConstantTimeEq;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::mfa::verify_email_otp::{command::Command, response::Response},
    },
    domain::{
        entities::{mfa_challenge::MfaChallengePurpose, mfa_method::MfaMethod},
        errors::DomainError,
    },
    utils::oauth_crypto::sha256_hex,
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
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

        let mut challenge = uow
            .mfa_challenge_repository()
            .find_active_by_session(session.id(), &MfaMethod::Email)
            .await?
            .filter(|c| *c.purpose() == MfaChallengePurpose::Login && c.is_redeemable(now))
            .ok_or_else(invalid)?;

        let expected = challenge.code_hash().ok_or_else(invalid)?;
        let submitted = sha256_hex(command.code.trim());
        if expected.as_bytes().ct_eq(submitted.as_bytes()).unwrap_u8() != 1 {
            challenge.register_failed_attempt();
            uow.mfa_challenge_repository().update(&challenge).await?;
            uow.commit().await?;
            return Err(invalid());
        }

        challenge.consume(now);
        session.verify_mfa(&MfaMethod::Email, now);
        uow.mfa_challenge_repository().update(&challenge).await?;
        uow.auth_session_repository().update(&session).await?;
        uow.commit().await?;

        Ok(Response {
            session_code: command.session_code,
        })
    }
}
