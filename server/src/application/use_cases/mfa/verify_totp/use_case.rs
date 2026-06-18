use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::{secret_cipher::SecretCipher, unit_of_work::UnitOfWorkFactory},
        use_cases::mfa::verify_totp::{command::Command, response::Response},
    },
    domain::{entities::mfa_method::MfaMethod, errors::DomainError},
    utils::{oauth_crypto::sha256_hex, totp},
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    secret_cipher: Arc<dyn SecretCipher>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        secret_cipher: Arc<dyn SecretCipher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            secret_cipher,
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
            .filter(|c| c.is_active() && c.allows_mfa_method(&MfaMethod::Totp))
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
        let mut enrollment = match command.mfa_id {
            Some(id) => {
                uow.mfa_enrollment_repository()
                    .find_by_id(client.tenant_id(), id)
                    .await?
            }
            None => {
                uow.mfa_enrollment_repository()
                    .find_totp_by_user_id(user_id)
                    .await?
            }
        }
        .filter(|e| e.user_id() == user_id && *e.method() == MfaMethod::Totp && e.is_confirmed())
        .ok_or_else(invalid)?;

        let encrypted = enrollment.secret_enc().ok_or_else(invalid)?;
        let secret_base32 = self.secret_cipher.decrypt(encrypted)?;
        let secret = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret_base32)
            .ok_or_else(invalid)?;
        let matched_step =
            totp::verify(&secret, command.code.trim(), now.timestamp(), 1).ok_or_else(invalid)?;

        if enrollment
            .totp_last_step()
            .map(|last| matched_step <= last)
            .unwrap_or(false)
        {
            return Err(invalid());
        }

        enrollment.record_totp_use(matched_step, now);
        session.verify_mfa(&MfaMethod::Totp, now);
        uow.mfa_enrollment_repository().update(&enrollment).await?;
        uow.auth_session_repository().update(&session).await?;
        uow.commit().await?;

        Ok(Response {
            session_code: command.session_code,
        })
    }
}
