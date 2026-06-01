
use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::user::User, errors::DomainError};

#[async_trait]
pub trait UserRepository: Send + Sync {
  async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, DomainError>;
  async fn find_by_id_with_profile(&self, user_id: Uuid) -> Result<Option<User>, DomainError>;
  async fn find_by_id_with_credentials(&self, user_id: Uuid) -> Result<Option<User>, DomainError>;
  
  async fn find_by_email(&self, tenant_id: Uuid, email: &str) -> Result<Option<User>, DomainError>;
  async fn find_by_tenant_id(&self, tenant_id: Uuid, page: u32, page_size: u32) -> Result<Vec<User>, DomainError>;

  async fn quantity_of_users_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError>;
  async fn is_user_with_email_exists(&self, tenant_id: Uuid, email: &str) -> Result<bool, DomainError>;

  async fn delete_by_id(&self, user_id: Uuid) -> Result<(), DomainError>;

  async fn add(&self, user: &User) -> Result<(), DomainError>;
  async fn update(&self, user: &User) -> Result<(), DomainError>;
}