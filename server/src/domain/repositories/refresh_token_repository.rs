use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::refresh_token::RefreshToken, errors::DomainError};

// One signed-in session, as the session list shows it: an active refresh-token family.
// A read model over the existing rows, not an entity — a family has no row of its own.
// Deliberately carries no token ids or hashes, and the table stores no IP or user
// agent, so nothing sensitive can reach a caller through it.
#[derive(Debug, Clone, PartialEq)]
pub struct RefreshTokenFamily {
    pub family_id: Uuid,
    // The internal oauth_clients.id of the app the user signed in to.
    pub client_id: Uuid,
    pub scopes: Vec<String>,
    // The oldest token of the family: when the authorization code was exchanged.
    pub signed_in_at: DateTime<Utc>,
    // The newest token of the family: the last refresh (or the sign-in itself).
    pub last_used_at: DateTime<Utc>,
    // When the family's live token expires if it is not refreshed again.
    pub expires_at: DateTime<Utc>,
}

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
    // The user's signed-in sessions: every family whose newest token is still live
    // (not revoked, not rotated, not expired at `now`), newest activity first.
    async fn find_active_families_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<RefreshTokenFamily>, DomainError>;
    // Ends one session: revokes the family's live tokens, but only when the family
    // belongs to `user_id` — a caller cannot reach another user's family by its id.
    // Returns whether the user owns the family at all (revoked already or not), so the
    // caller can tell "not found" from "already ended".
    async fn revoke_family_of_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        family_id: Uuid,
    ) -> Result<bool, DomainError>;
    // Retention purge (WI-03): hard-delete tokens already expired before `before`.
    // Returns the number of rows removed.
    async fn delete_expired(&self, before: DateTime<Utc>) -> Result<u64, DomainError>;
}
