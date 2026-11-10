use std::sync::Arc;

use crate::{
    application::{
        services::admin_authenticator::AdminAuthenticator,
        shared::actor::Actor,
        use_cases::admin_organization::{
            acting_admin, colleague, reset_admin_mfa::command::Command,
        },
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_mfa_repository::AdminMfaRepository,
            admin_refresh_token_repository::AdminRefreshTokenRepository,
            admin_user_repository::AdminUserRepository,
            admin_web_session_repository::AdminWebSessionRepository,
        },
    },
};

// The recovery path for an admin who lost both their authenticator and their
// recovery codes: a colleague removes their second factor (needs the colleague's
// current password). It also ends all of the admin's sessions, so whoever is signed
// in as them must sign in again; if the organization requires two-step verification
// they then have to enroll again before doing anything else. Returns whether a
// factor was removed.
pub struct UseCase {
    admins: Arc<dyn AdminUserRepository>,
    factors: Arc<dyn AdminMfaRepository>,
    authenticator: Arc<AdminAuthenticator>,
    refresh_tokens: Arc<dyn AdminRefreshTokenRepository>,
    web_sessions: Arc<dyn AdminWebSessionRepository>,
}

impl UseCase {
    pub fn new(
        admins: Arc<dyn AdminUserRepository>,
        factors: Arc<dyn AdminMfaRepository>,
        authenticator: Arc<AdminAuthenticator>,
        refresh_tokens: Arc<dyn AdminRefreshTokenRepository>,
        web_sessions: Arc<dyn AdminWebSessionRepository>,
    ) -> Self {
        Self {
            admins,
            factors,
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
                "use your own account settings to change your two-step verification".to_string(),
            ));
        }
        let target = colleague(
            self.admins.as_ref(),
            command.organization_id,
            command.admin_id,
        )
        .await?;

        let had_factor = self.factors.find(target.id()).await?.is_some();
        self.factors.delete(target.id()).await?;
        self.refresh_tokens
            .revoke_all_for_admin(target.id())
            .await?;
        self.web_sessions
            .delete_all_for_admin(target.id(), None)
            .await?;
        Ok(had_factor)
    }
}
