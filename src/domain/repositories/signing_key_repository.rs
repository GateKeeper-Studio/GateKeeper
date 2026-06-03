use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::{signing_key::SigningKey}, errors::DomainError};

#[async_trait]
pub trait SigningKeyRepository: Send + Sync {
  async fn find_active(&self, tenant_id: Uuid) -> Result<Option<SigningKey>, DomainError>; // signs new tokens
  async fn find_published(&self, tenant_id: Uuid) -> Result<Vec<SigningKey>, DomainError>; // builds /jwks.json
  async fn add(&self, key: &SigningKey) -> Result<(), DomainError>;
  async fn update(&self, key: &SigningKey) -> Result<(), DomainError>;                      // status transitions
}