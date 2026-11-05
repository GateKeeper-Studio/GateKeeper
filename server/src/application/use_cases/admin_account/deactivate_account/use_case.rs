use std::sync::Arc;

use crate::application::use_cases::admin_account::deactivate_account::command::Command;
use crate::domain::{
    errors::DomainError,
    repositories::{
        admin_refresh_token_repository::AdminRefreshTokenRepository,
        admin_user_repository::AdminUserRepository,
    },
};

// LGPD-respecting self-service deactivation: a reversible soft-delete. The admin row
// is kept (so audit history still attributes its past actions), `is_active` is flipped
// off — which login and token refresh both reject — and every session is revoked so the
// account stops working immediately. Another admin can reactivate it later.
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
}

impl UseCase {
    pub fn new(
        admin_repository: Arc<dyn AdminUserRepository>,
        refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
    ) -> Self {
        Self {
            admin_repository,
            refresh_token_repository,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let mut admin = self
            .admin_repository
            .find_by_id(command.admin_id)
            .await?
            .ok_or_else(|| {
                DomainError::UserNotFound(format!("admin {} not found", command.admin_id))
            })?;

        // Already inactive → idempotent success.
        if !admin.is_active() {
            return Ok(());
        }

        // Never leave an organization with zero active admins — that would lock everyone
        // (including tenant management) out with no way back in.
        let active = self
            .admin_repository
            .count_active_by_organization(admin.organization_id())
            .await?;
        if active <= 1 {
            return Err(DomainError::AccessDenied(
                "cannot deactivate the last active admin of your organization".to_string(),
            ));
        }

        // Flip the authoritative gate first (refresh already honors it), then revoke
        // every live session so the still-valid access token can't be renewed.
        admin.deactivate();
        self.admin_repository.update(&admin).await?;
        self.refresh_token_repository
            .revoke_all_for_admin(admin.id())
            .await?;

        Ok(())
    }
}
