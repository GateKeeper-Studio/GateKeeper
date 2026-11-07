use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::admin_web_session::AdminWebSession, errors::DomainError};

// Persistence contract for admin console browser sessions. Global (no tenant scope),
// looked up by the SHA-256 hash of the cookie value, the only form ever stored.
#[async_trait]
pub trait AdminWebSessionRepository: Send + Sync {
    // Also clears expired rows: sign-ins are rare, so this keeps the table small
    // without a scheduler of its own.
    async fn add(&self, session: &AdminWebSession) -> Result<(), DomainError>;
    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminWebSession>, DomainError>;
    async fn touch(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), DomainError>;
    // Idempotent: deleting an unknown hash is not an error.
    async fn delete_by_token_hash(&self, token_hash: &str) -> Result<(), DomainError>;
}
