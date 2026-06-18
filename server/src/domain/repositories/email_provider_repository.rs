use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::email_provider::EmailProvider, errors::DomainError};

#[async_trait]
pub trait EmailProviderRepository: Send + Sync {
    // Admin CRUD lookups: by the internal id, always tenant-scoped.
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<EmailProvider>, DomainError>;
    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<EmailProvider>, DomainError>;
    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError>;

    // The tenant's default provider — used when an OAuth client hasn't picked one.
    async fn find_default_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Option<EmailProvider>, DomainError>;
    // Clears the current default so a new one can be set (single default per tenant).
    async fn clear_default_for_tenant(&self, tenant_id: Uuid) -> Result<(), DomainError>;

    async fn add(&self, provider: &EmailProvider) -> Result<(), DomainError>;
    async fn update(&self, provider: &EmailProvider) -> Result<(), DomainError>;
    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError>;
}
