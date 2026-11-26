use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::compliance::purge_expired_data::response::Response,
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_invitation_repository::AdminInvitationRepository,
            admin_refresh_token_repository::AdminRefreshTokenRepository,
            admin_web_session_repository::AdminWebSessionRepository,
        },
    },
};

// Retention enforcement (WI-03, storage limitation). Hard-deletes already-expired
// session/token/code rows in one transaction. Idempotent — re-running purges nothing
// new. The audit log is NEVER touched here (its retention is a separate, longer
// floor). `data.purged` is emitted at the handler layer with the run's counts.
// The platform-admin credential tables: refresh tokens, console sessions, invitations.
type AdminCredentialRepositories = (
    Arc<dyn AdminRefreshTokenRepository>,
    Arc<dyn AdminWebSessionRepository>,
    Arc<dyn AdminInvitationRepository>,
);

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    // Platform-admin credential tables. Not part of the tenant Unit of Work, so they
    // are purged on their own after it commits (each delete is idempotent).
    admin_sessions: Option<AdminCredentialRepositories>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
            admin_sessions: None,
        }
    }

    pub fn with_admin_sessions(
        mut self,
        refresh_tokens: Arc<dyn AdminRefreshTokenRepository>,
        console_sessions: Arc<dyn AdminWebSessionRepository>,
        invitations: Arc<dyn AdminInvitationRepository>,
    ) -> Self {
        self.admin_sessions = Some((refresh_tokens, console_sessions, invitations));
        self
    }

    pub async fn execute(&self) -> Result<Response, DomainError> {
        let now = Utc::now();
        let uow = self.unit_of_work_factory.begin().await?;

        let refresh_tokens = uow.refresh_token_repository().delete_expired(now).await?;
        let auth_sessions = uow.auth_session_repository().delete_expired(now).await?;
        let authorization_codes = uow
            .authorization_code_repository()
            .delete_expired(now)
            .await?;

        // Short-lived credential and challenge rows. Every one of these tables has an
        // `expires_at` and a migration comment promising a background job would clear
        // it; none was ever wired, so they accumulated indefinitely. An expired row
        // here is already unusable — `is_redeemable` and friends reject it — so
        // deleting it changes no behaviour, only storage.
        let email_verification_tokens = uow
            .email_verification_token_repository()
            .delete_expired(now)
            .await?;
        let password_reset_tokens = uow
            .password_reset_token_repository()
            .delete_expired(now)
            .await?;
        let password_change_tokens = uow
            .password_change_token_repository()
            .delete_expired(now)
            .await?;
        let mfa_challenges = uow.mfa_challenge_repository().delete_expired(now).await?;
        let external_login_flows = uow
            .external_login_flow_repository()
            .delete_expired(now)
            .await?;
        // Expired IP blocks: the risk engine already ignores them (it filters on
        // expires_at), so removing them frees storage without unblocking anyone.
        let security_blocks = uow.security_block_repository().delete_expired(now).await?;

        uow.commit().await?;

        let (admin_refresh_tokens, admin_console_sessions, admin_invitations) =
            match &self.admin_sessions {
                Some((refresh, console, invitations)) => (
                    refresh.delete_expired(now).await?,
                    console.delete_expired(now).await?,
                    invitations.delete_expired(now).await?,
                ),
                None => (0, 0, 0),
            };

        let total = refresh_tokens
            + auth_sessions
            + authorization_codes
            + email_verification_tokens
            + password_reset_tokens
            + password_change_tokens
            + mfa_challenges
            + external_login_flows
            + security_blocks
            + admin_refresh_tokens
            + admin_console_sessions
            + admin_invitations;

        Ok(Response {
            refresh_tokens,
            auth_sessions,
            authorization_codes,
            email_verification_tokens,
            password_reset_tokens,
            password_change_tokens,
            mfa_challenges,
            external_login_flows,
            security_blocks,
            admin_refresh_tokens,
            admin_console_sessions,
            admin_invitations,
            total,
        })
    }
}
