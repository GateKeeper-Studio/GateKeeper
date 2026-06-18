use std::sync::Arc;

use rand::RngCore;

use crate::{
    application::{
        ports::{secret_cipher::SecretCipher, unit_of_work::UnitOfWorkFactory},
        use_cases::mfa::start_totp_enrollment::{command::Command, response::Response},
    },
    domain::{entities::mfa_enrollment::MfaEnrollment, errors::DomainError},
    utils::totp,
};

// 160-bit shared secret, the RFC 4226 recommended size for HMAC-SHA1.
const SECRET_BYTES: usize = 20;

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

        let user = uow
            .user_repository()
            .find_by_id(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id && u.is_active())
            .ok_or_else(|| DomainError::UserNotFound("user not found".to_string()))?;

        // Issuer shown by the authenticator app next to the account label.
        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| DomainError::TenantNotFound("tenant not found".to_string()))?;

        // One TOTP enrollment per user (DB partial unique index). A confirmed
        // authenticator is a working factor — never silently invalidate it; an
        // abandoned pending enrollment is safe to replace with a fresh secret.
        if let Some(existing) = uow
            .mfa_enrollment_repository()
            .find_totp_by_user_id(command.user_id)
            .await?
        {
            if existing.is_confirmed() {
                return Err(DomainError::MfaEnrollmentConflict(
                    "an authenticator app is already enrolled; remove it before adding a new one"
                        .to_string(),
                ));
            }
            uow.mfa_enrollment_repository()
                .delete_by_id(existing.tenant_id(), existing.id())
                .await?;
        }

        let mut secret = [0u8; SECRET_BYTES];
        rand::rng().fill_bytes(&mut secret);
        let secret_base32 = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret);
        let secret_enc = self.secret_cipher.encrypt(&secret_base32)?;

        let enrollment = MfaEnrollment::new_totp(command.tenant_id, command.user_id, secret_enc);
        let enrollment_id = enrollment.id();
        uow.mfa_enrollment_repository().add(&enrollment).await?;
        uow.commit().await?;

        Ok(Response {
            enrollment_id,
            otpauth_url: totp::otpauth_url(tenant.name(), user.email(), &secret_base32),
            secret_base32,
        })
    }
}
