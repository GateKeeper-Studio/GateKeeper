use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::trusted_device::TrustedDevice, errors::DomainError};

// Every lookup is scoped to the tenant AND the user: a device id alone never reaches
// another account's device.
#[async_trait]
pub trait TrustedDeviceRepository: Send + Sync {
    async fn add(&self, device: &TrustedDevice) -> Result<(), DomainError>;
    async fn find(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Option<TrustedDevice>, DomainError>;
    // Oldest first.
    async fn list_for_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<TrustedDevice>, DomainError>;
    async fn count_for_user(&self, tenant_id: Uuid, user_id: Uuid) -> Result<i64, DomainError>;
    async fn has_key(&self, user_id: Uuid, public_key: &str) -> Result<bool, DomainError>;
    async fn record_use(&self, id: Uuid, at: DateTime<Utc>) -> Result<(), DomainError>;
    async fn delete(&self, tenant_id: Uuid, user_id: Uuid, id: Uuid) -> Result<bool, DomainError>;
    // When the user's credentials change or every session is ended: a device trusted
    // under the old password must not outlive it. Returns how many were removed.
    async fn delete_all_for_user(&self, tenant_id: Uuid, user_id: Uuid)
    -> Result<u64, DomainError>;
}
