use std::sync::Arc;

use chrono::Utc;
use rand::RngCore;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        use_cases::admin_mfa::start_totp::{command::Command, response::Response},
    },
    domain::{
        entities::admin_mfa::AdminMfa,
        errors::DomainError,
        repositories::{
            admin_mfa_repository::AdminMfaRepository, admin_user_repository::AdminUserRepository,
        },
    },
    utils::totp,
};

// 160-bit shared secret, the RFC 4226 recommended size for HMAC-SHA1.
const SECRET_BYTES: usize = 20;

// What authenticator apps show above the account.
const ISSUER: &str = "GateKeeper";

// Starts enrolling an authenticator app: a fresh seed, sealed, stored as pending.
// Nothing changes at sign-in until `confirm_totp` proves the app has the seed. An
// earlier abandoned enrollment is replaced; an enabled factor never is.
pub struct UseCase {
    admins: Arc<dyn AdminUserRepository>,
    factors: Arc<dyn AdminMfaRepository>,
    authenticator: Arc<AdminAuthenticator>,
}

impl UseCase {
    pub fn new(
        admins: Arc<dyn AdminUserRepository>,
        factors: Arc<dyn AdminMfaRepository>,
        authenticator: Arc<AdminAuthenticator>,
    ) -> Self {
        Self {
            admins,
            factors,
            authenticator,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let admin = self
            .admins
            .find_by_id(command.admin_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("admin not found".to_string()))?;
        self.authenticator
            .require_password(&admin, command.current_password.as_deref())
            .await?;

        let already_enabled = || {
            DomainError::Conflict(
                "two-step verification is already on; turn it off before enrolling again"
                    .to_string(),
            )
        };
        if self
            .factors
            .find(admin.id())
            .await?
            .is_some_and(|f| f.is_enabled())
        {
            return Err(already_enabled());
        }

        let mut secret = [0u8; SECRET_BYTES];
        rand::rng().fill_bytes(&mut secret);
        let secret_base32 = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret);
        let sealed = self
            .authenticator
            .seal_totp_seed(admin.id(), &secret_base32)?;

        if !self
            .factors
            .save_pending(&AdminMfa::pending(admin.id(), sealed, Utc::now()))
            .await?
        {
            return Err(already_enabled());
        }

        Ok(Response {
            otpauth_url: totp::otpauth_url(ISSUER, admin.email(), &secret_base32),
            secret_base32,
        })
    }
}
