use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        services::admin_authenticator::{AdminAuthenticator, generate_recovery_codes},
        use_cases::admin_mfa::regenerate_recovery_codes::{command::Command, response::Response},
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_mfa_repository::AdminMfaRepository, admin_user_repository::AdminUserRepository,
        },
    },
};

// Replaces every recovery code with a fresh set (after the old ones ran low or may
// have been seen). Requires the current password.
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

        let (codes, hashes) = generate_recovery_codes();
        if !self
            .factors
            .replace_recovery_codes(admin.id(), &hashes, Utc::now())
            .await?
        {
            return Err(DomainError::Validation(
                "two-step verification is not on".to_string(),
            ));
        }
        Ok(Response {
            recovery_codes: codes,
        })
    }
}
