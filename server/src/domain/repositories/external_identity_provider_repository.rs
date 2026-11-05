use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::external_identity_provider::ExternalIdentityProvider, errors::DomainError,
};

#[async_trait]
pub trait ExternalIdentityProviderRepository: Send + Sync {
    // Admin CRUD: all providers configured for a client (active or not).
    async fn find_by_oauth_client_id(
        &self,
        oauth_client_id: Uuid,
    ) -> Result<Vec<ExternalIdentityProvider>, DomainError>;

    // Public sign-in screen: only the active providers a client offers.
    async fn find_active_by_oauth_client_id(
        &self,
        oauth_client_id: Uuid,
    ) -> Result<Vec<ExternalIdentityProvider>, DomainError>;

    // Admin CRUD by internal id, tenant-scoped.
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<ExternalIdentityProvider>, DomainError>;

    // Public login flow: resolve an active provider by its id without a tenant
    // context (the SPA only knows the provider id).
    async fn find_active_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<ExternalIdentityProvider>, DomainError>;

    async fn add(&self, provider: &ExternalIdentityProvider) -> Result<(), DomainError>;
    async fn update(&self, provider: &ExternalIdentityProvider) -> Result<(), DomainError>;
    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError>;
}
