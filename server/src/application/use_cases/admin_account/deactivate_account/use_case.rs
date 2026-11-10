use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        use_cases::admin_account::deactivate_account::command::Command,
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_refresh_token_repository::AdminRefreshTokenRepository,
            admin_user_repository::AdminUserRepository,
            admin_web_session_repository::AdminWebSessionRepository,
        },
    },
};

// LGPD-respecting self-service deactivation: a soft delete. The admin row is kept
// (so the audit log still attributes its past actions), `is_active` is flipped
// off, which sign-in, refresh and console sessions all reject, and every session
// is ended so the account stops working at once. Another admin of the organization
// can reactivate it (`admin_organization::set_admin_active`).
//
// Requires the current password (step-up), and never deactivates the last active
// admin of an organization, which would leave nobody able to manage it.
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    authenticator: Arc<AdminAuthenticator>,
    refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
    web_session_repository: Arc<dyn AdminWebSessionRepository>,
}

impl UseCase {
    pub fn new(
        admin_repository: Arc<dyn AdminUserRepository>,
        authenticator: Arc<AdminAuthenticator>,
        refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
        web_session_repository: Arc<dyn AdminWebSessionRepository>,
    ) -> Self {
        Self {
            admin_repository,
            authenticator,
            refresh_token_repository,
            web_session_repository,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let admin = self
            .admin_repository
            .find_by_id(command.admin_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("admin {} not found", command.admin_id))
            })?;

        // Already inactive: idempotent success.
        if !admin.is_active() {
            return Ok(());
        }

        self.authenticator
            .require_password(&admin, command.current_password.as_deref())
            .await?;

        if !self
            .admin_repository
            .deactivate_unless_last_active(&admin, Utc::now())
            .await?
        {
            return Err(DomainError::AccessDenied(
                "cannot deactivate the last active admin of your organization".to_string(),
            ));
        }

        self.refresh_token_repository
            .revoke_all_for_admin(admin.id())
            .await?;
        self.web_session_repository
            .delete_all_for_admin(admin.id(), None)
            .await?;

        Ok(())
    }
}
