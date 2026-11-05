use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::webhook_subscription::WebhookSubscription, errors::DomainError};

#[async_trait]
pub trait WebhookSubscriptionRepository: Send + Sync {
    // Admin CRUD lookups: always tenant-scoped, so one tenant can never read or
    // edit another's delivery targets.
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<WebhookSubscription>, DomainError>;

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<WebhookSubscription>, DomainError>;

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError>;

    // Dispatcher fan-out: the active targets for one tenant's event.
    async fn find_active_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<WebhookSubscription>, DomainError>;

    async fn add(&self, subscription: &WebhookSubscription) -> Result<(), DomainError>;
    async fn update(&self, subscription: &WebhookSubscription) -> Result<(), DomainError>;
    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError>;
}
