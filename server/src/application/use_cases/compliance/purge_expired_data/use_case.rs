use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::compliance::purge_expired_data::response::Response,
    },
    domain::errors::DomainError,
};

// Retention enforcement (WI-03, storage limitation). Hard-deletes already-expired
// session/token/code rows in one transaction. Idempotent — re-running purges nothing
// new. The audit log is NEVER touched here (its retention is a separate, longer
// floor). `data.purged` is emitted at the handler layer with the run's counts.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
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

        let total = refresh_tokens
            + auth_sessions
            + authorization_codes
            + email_verification_tokens
            + password_reset_tokens
            + password_change_tokens
            + mfa_challenges
            + external_login_flows
            + security_blocks;

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
            total,
        })
    }
}
