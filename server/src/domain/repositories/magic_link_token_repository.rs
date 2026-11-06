use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::magic_link_token::MagicLinkToken, errors::DomainError};

#[async_trait]
pub trait MagicLinkTokenRepository: Send + Sync {
    // Links carry their row id (`&id=`), so redemption looks the token up by id and
    // then hash-compares the secret. The high-entropy token is never a lookup key.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<MagicLinkToken>, DomainError>;
    async fn add(&self, token: &MagicLinkToken) -> Result<(), DomainError>;
    // Persists redemption (consumed_at).
    async fn update(&self, token: &MagicLinkToken) -> Result<(), DomainError>;
    // When the newest link for this user was minted, for the resend cooldown. There
    // is deliberately no "invalidate every link of this user": anyone who knows an
    // email can request a link, so a per-user invalidation would let them cancel the
    // real user's pending sign-in. Each link only works with its own session anyway.
    async fn latest_created_at_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, DomainError>;
}
