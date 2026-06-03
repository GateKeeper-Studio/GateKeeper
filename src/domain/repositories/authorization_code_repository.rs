use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::authorization_code::AuthorizationCode, errors::DomainError};

#[async_trait]
pub trait AuthorizationCodeRepository: Send + Sync {
  async fn find_by_code_hash(&self, tenant_id: Uuid, code_hash: &str) -> Result<Option<AuthorizationCode>, DomainError>;
  async fn add(&self, code: &AuthorizationCode) -> Result<(), DomainError>;
  async fn mark_consumed(&self, code: &AuthorizationCode) -> Result<(), DomainError>;
}