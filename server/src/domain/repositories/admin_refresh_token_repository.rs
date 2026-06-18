use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::admin_refresh_token::AdminRefreshToken, errors::DomainError};

// Persistence contract for platform-admin refresh tokens. Global (no tenant scope):
// lookups are by the token's SHA-256 hash, the only thing we ever store.
#[async_trait]
pub trait AdminRefreshTokenRepository: Send + Sync {
    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminRefreshToken>, DomainError>;
    async fn add(&self, token: &AdminRefreshToken) -> Result<(), DomainError>;
    // Stamp the old token as rotated and persist its successor (one transaction).
    async fn rotate(
        &self,
        old: &AdminRefreshToken,
        new: &AdminRefreshToken,
    ) -> Result<(), DomainError>;
    // Reuse-detection kill switch + server-side logout: revoke every live token in a family.
    async fn revoke_family(&self, family_id: Uuid) -> Result<(), DomainError>;
}
