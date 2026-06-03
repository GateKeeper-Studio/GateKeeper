use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::oauth_client::OAuthClient, errors::DomainError};

#[async_trait]
pub trait OAuthClientRepository: Send + Sync {
  // Protocol lookups (used by the OAuth flow): by the PUBLIC client_id.
  async fn find_by_client_id(&self, tenant_id: Uuid, client_id: &str) -> Result<Option<OAuthClient>, DomainError>;

  // Admin CRUD lookups: by the INTERNAL id, always tenant-scoped.
  async fn find_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<OAuthClient>, DomainError>;
  async fn find_by_tenant_id(&self, tenant_id: Uuid, page: u32, page_size: u32) -> Result<Vec<OAuthClient>, DomainError>;
  async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError>;

  async fn add(&self, client: &OAuthClient) -> Result<(), DomainError>;
  async fn update(&self, client: &OAuthClient) -> Result<(), DomainError>;
  async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError>;
}
