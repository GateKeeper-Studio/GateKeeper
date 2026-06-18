use chrono::{DateTime, Utc};
use uuid::Uuid;

// Proof-of-control credential minted when a user asks to reset a forgotten
// password and mailed to them as a one-time link. Possessing the link's token
// authorizes setting a new password without knowing the old one.
//
// SECURITY: never store the raw token. We persist SHA-256(token); the raw value
// (256 bits of entropy from `generate_opaque_token`) lives only in the emailed
// link and is shown exactly once. The token is single-use (consumed_at) and
// short-lived (expires_at). Lookup is by the row `id` (carried as `&id=` in the
// link), so the high-entropy secret is never itself a SQL key. Unlike the
// 6-digit email-confirmation code, a 256-bit token is not brute-forceable, so no
// attempts counter is needed here.
#[derive(Debug, Clone)]
pub struct PasswordResetToken {
    id: Uuid,
    tenant_id: Uuid,
    user_id: Uuid,

    token_hash: String,

    consumed_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl PasswordResetToken {
    pub fn new(
        tenant_id: Uuid,
        user_id: Uuid,
        token_hash: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        PasswordResetToken {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            token_hash,
            consumed_at: None,
            expires_at,
            created_at: Utc::now(),
        }
    }

    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        token_hash: String,
        consumed_at: Option<DateTime<Utc>>,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        PasswordResetToken {
            id,
            tenant_id,
            user_id,
            token_hash,
            consumed_at,
            expires_at,
            created_at,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    pub fn is_consumed(&self) -> bool {
        self.consumed_at.is_some()
    }

    pub fn is_redeemable(&self, now: DateTime<Utc>) -> bool {
        !self.is_consumed() && !self.is_expired(now)
    }

    // Single-use enforcement: stamp the redemption time.
    pub fn consume(&mut self, now: DateTime<Utc>) {
        self.consumed_at = Some(now);
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn token_hash(&self) -> &str {
        &self.token_hash
    }
    pub fn consumed_at(&self) -> Option<DateTime<Utc>> {
        self.consumed_at
    }
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
