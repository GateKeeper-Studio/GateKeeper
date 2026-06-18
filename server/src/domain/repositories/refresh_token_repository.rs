use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::refresh_token::RefreshToken, errors::DomainError};

#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn find_by_token_hash(
        &self,
        tenant_id: Uuid,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, DomainError>;
    async fn add(&self, token: &RefreshToken) -> Result<(), DomainError>;
    async fn rotate(&self, old: &RefreshToken, new: &RefreshToken) -> Result<(), DomainError>;
    async fn revoke_family(&self, tenant_id: Uuid, family_id: Uuid) -> Result<(), DomainError>; // reuse-detection kill switch
    // Cuts off all live refresh tokens a user holds for one client — the side
    // effect of revoking that client's consent.
    async fn revoke_by_user_and_client(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        client_id: Uuid,
    ) -> Result<(), DomainError>;
    // Cuts off every live refresh token a user holds across all clients — used
    // after a password reset to log the account out everywhere.
    async fn revoke_by_user(&self, tenant_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    // Retention purge (WI-03): hard-delete tokens already expired before `before`.
    // Returns the number of rows removed.
    async fn delete_expired(&self, before: DateTime<Utc>) -> Result<u64, DomainError>;
}
