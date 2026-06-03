use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::auth_session::AuthSession, errors::DomainError};

#[async_trait]
pub trait AuthSessionRepository: Send + Sync {
  async fn find_by_id(&self, tenant_id: Uuid, session_id: Uuid) -> Result<Option<AuthSession>, DomainError>;
  async fn add(&self, session: &AuthSession) -> Result<(), DomainError>;
  async fn update(&self, session: &AuthSession) -> Result<(), DomainError>;
}