use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::security_block::SecurityBlock, errors::DomainError};

// Temporary IP/user blocks. Reads are tenant-scoped. "Active" means `expires_at` is
// in the future — expired rows are left in place for history and filtered out here.
#[async_trait]
pub trait SecurityBlockRepository: Send + Sync {
    async fn add(&self, block: &SecurityBlock) -> Result<(), DomainError>;

    async fn find_active_ip_block(
        &self,
        tenant_id: Uuid,
        ip: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<SecurityBlock>, DomainError>;

    async fn find_active_user_block(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Option<SecurityBlock>, DomainError>;

    async fn list_active(
        &self,
        tenant_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<SecurityBlock>, DomainError>;

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<SecurityBlock>, DomainError>;

    // Removes the block. Returns true if a row was removed (false ⇒ not found), so
    // the caller can surface a 404 on an unknown id.
    async fn delete(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, DomainError>;

    // Retention: hard-delete rows already past `expires_at`. Called by the
    // background purge; idempotent, so concurrent runs are harmless.
    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError>;
}
