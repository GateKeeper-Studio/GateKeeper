use async_trait::async_trait;
use chrono::{DateTime, Utc};
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
    // Stamps the old token as rotated and persists its successor, in one statement
    // that only succeeds while the old token is still live. Returns false when
    // another request rotated or revoked it first: two concurrent refreshes with
    // the same token can never both mint a successor.
    async fn rotate(
        &self,
        old: &AdminRefreshToken,
        new: &AdminRefreshToken,
    ) -> Result<bool, DomainError>;
    // Reuse-detection kill switch + server-side logout: revoke every live token in a family.
    async fn revoke_family(&self, family_id: Uuid) -> Result<(), DomainError>;
    // Ends every session for one admin at once — used when the account is deactivated,
    // so all rotation families (not just the current one) stop working immediately.
    async fn revoke_all_for_admin(&self, admin_id: Uuid) -> Result<(), DomainError>;
    // Retention: hard-deletes tokens past their expiry. Returns how many.
    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64, DomainError>;
}
