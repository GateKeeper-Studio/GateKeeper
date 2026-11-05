use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::external_login_flow::ExternalLoginFlow, errors::DomainError};

#[async_trait]
pub trait ExternalLoginFlowRepository: Send + Sync {
    async fn add(&self, flow: &ExternalLoginFlow) -> Result<(), DomainError>;

    // Look up a pending flow by the SHA-256 hash of the state echoed by the provider.
    async fn find_by_state_hash(
        &self,
        state_hash: &str,
    ) -> Result<Option<ExternalLoginFlow>, DomainError>;

    // Single-use: remove the flow once it has been consumed by the callback.
    async fn delete_by_id(&self, id: Uuid) -> Result<(), DomainError>;

    // Retention: hard-delete rows already past `expires_at`. Called by the
    // background purge; idempotent, so concurrent runs are harmless.
    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError>;
}
