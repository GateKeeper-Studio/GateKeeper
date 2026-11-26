use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        shared::actor::Actor,
        use_cases::admin_organization::{
            acting_admin, colleague, set_admin_active::command::Command,
        },
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

// Deactivates or reactivates another admin of the organization. Deactivating ends
// all of their sessions and is refused for the last active admin; an admin
// deactivates themselves through `admin_account::deactivate_account`. Needs the
// current password. Returns whether anything changed.
pub struct UseCase {
    admins: Arc<dyn AdminUserRepository>,
    authenticator: Arc<AdminAuthenticator>,
    refresh_tokens: Arc<dyn AdminRefreshTokenRepository>,
    web_sessions: Arc<dyn AdminWebSessionRepository>,
}

impl UseCase {
    pub fn new(
        admins: Arc<dyn AdminUserRepository>,
        authenticator: Arc<AdminAuthenticator>,
        refresh_tokens: Arc<dyn AdminRefreshTokenRepository>,
        web_sessions: Arc<dyn AdminWebSessionRepository>,
    ) -> Self {
        Self {
            admins,
            authenticator,
            refresh_tokens,
            web_sessions,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<bool, DomainError> {
        let me = acting_admin(actor, self.admins.as_ref(), command.organization_id).await?;
        self.authenticator
            .require_password(&me, command.current_password.as_deref())
            .await?;
        if command.admin_id == me.id() {
            return Err(DomainError::Validation(
                "use your own account settings to deactivate yourself".to_string(),
            ));
        }
        let target = colleague(
            self.admins.as_ref(),
            command.organization_id,
            command.admin_id,
        )
        .await?;
        let now = Utc::now();

        if command.active {
            if target.is_active() {
                return Ok(false);
            }
            return self
                .admins
                .reactivate(target.id(), command.organization_id, now)
                .await;
        }

        if !target.is_active() {
            return Ok(false);
        }
        if !self
            .admins
            .deactivate_unless_last_active(&target, now)
            .await?
        {
            return Err(DomainError::AccessDenied(
                "cannot deactivate the last active admin of the organization".to_string(),
            ));
        }
        self.refresh_tokens
            .revoke_all_for_admin(target.id())
            .await?;
        self.web_sessions
            .delete_all_for_admin(target.id(), None)
            .await?;
        Ok(true)
    }
}
