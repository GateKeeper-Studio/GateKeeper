use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::authorization_code::AuthorizationCode, errors::DomainError};

#[async_trait]
pub trait AuthorizationCodeRepository: Send + Sync {
    async fn find_by_code_hash(
        &self,
        tenant_id: Uuid,
        code_hash: &str,
    ) -> Result<Option<AuthorizationCode>, DomainError>;
    async fn add(&self, code: &AuthorizationCode) -> Result<(), DomainError>;
    async fn mark_consumed(&self, code: &AuthorizationCode) -> Result<(), DomainError>;
    // Retention purge (WI-03): hard-delete codes expired before `before`.
    async fn delete_expired(&self, before: DateTime<Utc>) -> Result<u64, DomainError>;
}
