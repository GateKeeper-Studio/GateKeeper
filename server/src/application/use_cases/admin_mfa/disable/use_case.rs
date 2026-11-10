use std::sync::Arc;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        use_cases::admin_mfa::disable::command::Command,
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_mfa_repository::AdminMfaRepository, admin_user_repository::AdminUserRepository,
        },
    },
};

// Turns two-step verification off. Needs both the password and a second-factor code,
// so neither a stolen session nor a leaked password alone can remove the factor.
// Returns whether a factor that was in force got removed (for the audit log).
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

    pub async fn execute(&self, command: Command) -> Result<bool, DomainError> {
        let admin = self
            .admins
            .find_by_id(command.admin_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("admin not found".to_string()))?;
        self.authenticator
            .require_password(&admin, command.current_password.as_deref())
            .await?;

        let Some(factor) = self.factors.find(admin.id()).await? else {
            return Ok(false);
        };
        if factor.is_enabled() {
            self.authenticator
                .require_second_factor(&admin, command.code.as_deref())
                .await?;
        }
        self.factors.delete(admin.id()).await?;
        Ok(factor.is_enabled())
    }
}
