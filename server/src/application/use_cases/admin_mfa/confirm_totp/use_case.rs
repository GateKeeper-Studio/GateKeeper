use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        services::admin_authenticator::{AdminAuthenticator, generate_recovery_codes},
        use_cases::admin_mfa::confirm_totp::{command::Command, response::Response},
    },
    domain::{errors::DomainError, repositories::admin_mfa_repository::AdminMfaRepository},
};

// Puts a pending authenticator app in force once it produces a valid code, and
// hands out the first set of recovery codes. From then on sign-in asks for a code.
pub struct UseCase {
    factors: Arc<dyn AdminMfaRepository>,
    authenticator: Arc<AdminAuthenticator>,
}

impl UseCase {
    pub fn new(
        factors: Arc<dyn AdminMfaRepository>,
        authenticator: Arc<AdminAuthenticator>,
    ) -> Self {
        Self {
            factors,
            authenticator,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let no_pending = || {
            DomainError::Conflict(
                "there is no authenticator app waiting to be confirmed; start again".to_string(),
            )
        };
        let factor = self
            .factors
            .find(command.admin_id)
            .await?
            .filter(|f| !f.is_enabled())
            .ok_or_else(no_pending)?;

        let now = Utc::now();
        let step = self
            .authenticator
            .match_totp(&factor, command.code.trim(), now)
            .ok_or_else(|| DomainError::Validation("the code is not valid".to_string()))?;

        let (codes, hashes) = generate_recovery_codes();
        if !self
            .factors
            .confirm(command.admin_id, step, &hashes, now)
            .await?
        {
            return Err(no_pending());
        }
        Ok(Response {
            recovery_codes: codes,
        })
    }
}
