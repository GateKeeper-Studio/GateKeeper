use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::auth_session::AuthSession, errors::DomainError};

#[async_trait]
pub trait AuthSessionRepository: Send + Sync {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        session_id: Uuid,
    ) -> Result<Option<AuthSession>, DomainError>;
    // Look up a session by the SHA-256 hash of its opaque token — the lookup key the
    // login SPA presents (as the raw `sessionCode`) when completing the flow.
    async fn find_by_token_hash(
        &self,
        tenant_id: Uuid,
        token_hash: &str,
    ) -> Result<Option<AuthSession>, DomainError>;
    async fn add(&self, session: &AuthSession) -> Result<(), DomainError>;
    async fn update(&self, session: &AuthSession) -> Result<(), DomainError>;
    // Remove all sessions belonging to a user — used by erasure (WI-06).
    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError>;
    // Retention purge (WI-03): hard-delete sessions expired before `before`.
    async fn delete_expired(&self, before: DateTime<Utc>) -> Result<u64, DomainError>;
}
