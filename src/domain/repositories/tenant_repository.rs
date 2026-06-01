use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::tenant::Tenant, errors::DomainError};

#[async_trait]
pub trait TenantRepository: Send + Sync {
  async fn find_by_id(&self, tenant_id: Uuid) -> Result<Option<Tenant>, DomainError>;
  async fn find_all(&self, page: u32, page_size: u32) -> Result<Vec<Tenant>, DomainError>;
  async fn count_all(&self) -> Result<u32, DomainError>;
  async fn is_tenant_with_name_exists(&self, name: &str) -> Result<bool, DomainError>;

  async fn delete_by_id(&self, tenant_id: Uuid) -> Result<(), DomainError>;

  async fn add(&self, tenant: &Tenant) -> Result<(), DomainError>;
  async fn update(&self, tenant: &Tenant) -> Result<(), DomainError>;
}