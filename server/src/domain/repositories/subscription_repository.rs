use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::subscription::Subscription, errors::DomainError};

#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    async fn find_by_id(&self, subscription_id: Uuid) -> Result<Option<Subscription>, DomainError>;
    // An organization owns at most one subscription, so this returns a single row.
    async fn find_by_organization_id(
        &self,
        organization_id: Uuid,
    ) -> Result<Option<Subscription>, DomainError>;
    async fn find_all(&self, page: u32, page_size: u32) -> Result<Vec<Subscription>, DomainError>;
    async fn count_all(&self) -> Result<u32, DomainError>;

    // How many tenants belong to a subscription — lets a use case enforce the
    // plan's `max_tenants` limit before adding another.
    async fn count_tenants(&self, subscription_id: Uuid) -> Result<u32, DomainError>;

    // The largest per-tenant user count across all of the subscription's tenants
    // (0 when it has no tenants). Lets a plan change enforce the target plan's
    // `max_users` (per-tenant) limit before downgrading.
    async fn max_users_in_any_tenant(&self, subscription_id: Uuid)
    -> Result<u32, DomainError>;

    async fn delete_by_id(&self, subscription_id: Uuid) -> Result<(), DomainError>;

    async fn add(&self, subscription: &Subscription) -> Result<(), DomainError>;
    async fn update(&self, subscription: &Subscription) -> Result<(), DomainError>;
}
