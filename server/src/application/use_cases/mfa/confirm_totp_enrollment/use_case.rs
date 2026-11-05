use std::sync::Arc;

use chrono::Utc;

use crate::application::ports::secret_cipher::{SecretContext, SecretPurpose};
use crate::{
    application::{
        ports::{secret_cipher::SecretCipher, unit_of_work::UnitOfWorkFactory},
        use_cases::mfa::confirm_totp_enrollment::{command::Command, response::Response},
    },
    domain::errors::DomainError,
    utils::totp,
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
        let invalid = || DomainError::MfaError("invalid code".to_string());

        let mut enrollment = uow
            .mfa_enrollment_repository()
            .find_totp_by_user_id(command.user_id)
            .await?
            .filter(|e| e.tenant_id() == command.tenant_id && !e.is_confirmed())
            .ok_or_else(|| {
                DomainError::NotFound("no pending authenticator enrollment".to_string())
            })?;

        let encrypted = enrollment.secret_enc().ok_or_else(invalid)?;
        let secret_base32 = self.secret_cipher.decrypt(
            encrypted,
            SecretContext::new(SecretPurpose::TotpSeed, enrollment.tenant_id()),
        )?;
        let secret = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret_base32)
            .ok_or_else(invalid)?;
        let matched_step =
            totp::verify(&secret, command.code.trim(), now.timestamp(), 1).ok_or_else(invalid)?;

        // Seed the replay guard with the confirmation step so the same code
        // cannot be replayed at the next login inside its 30-second window.
        enrollment.confirm(now);
        enrollment.record_totp_use(matched_step, now);
        let enrollment_id = enrollment.id();
        uow.mfa_enrollment_repository().update(&enrollment).await?;
        uow.commit().await?;

        Ok(Response {
            enrollment_id,
            confirmed_at: now,
        })
    }
}
