use chrono::{DateTime, Utc};
use uuid::Uuid;

// Proof-of-control credential minted at self-registration and mailed to the user
// as a short 6-digit code. Typing it back confirms the user owns the email and
// lets the auth flow proceed.
//
// SECURITY: never store the raw code. We persist SHA-256(code); the raw value is
// emailed exactly once and never written down. The code is single-use
// (consumed_at) and short-lived (expires_at). Because a 6-digit code has only
// 10^6 combinations it is brute-forceable, so redemption is scoped to the user
// (looked up by email, not by code) and capped at MAX_ATTEMPTS wrong tries. The
// pending OAuth request is NOT stored here — it is re-supplied and re-validated
// at confirm time, so this row stays minimal.
#[derive(Debug, Clone)]
pub struct EmailVerificationToken {
    id: Uuid,
    tenant_id: Uuid,
    user_id: Uuid,

    token_hash: String,

    attempts: i32,
    consumed_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl EmailVerificationToken {
    // Wrong-code tries allowed before the code is burned; the user must request a
    // fresh one via resend.
    pub const MAX_ATTEMPTS: i32 = 5;

    pub fn new(
        tenant_id: Uuid,
        user_id: Uuid,
        token_hash: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        EmailVerificationToken {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            token_hash,
            attempts: 0,
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
        attempts: i32,
        consumed_at: Option<DateTime<Utc>>,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        EmailVerificationToken {
            id,
            tenant_id,
            user_id,
            token_hash,
            attempts,
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

    // Too many wrong codes were submitted — the code is dead even if not yet
    // expired or consumed.
    pub fn attempts_exhausted(&self) -> bool {
        self.attempts >= Self::MAX_ATTEMPTS
    }

    pub fn is_redeemable(&self, now: DateTime<Utc>) -> bool {
        !self.is_consumed() && !self.is_expired(now) && !self.attempts_exhausted()
    }

    // Count a wrong-code submission toward the lockout threshold.
    pub fn register_failed_attempt(&mut self) {
        self.attempts += 1;
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
    pub fn attempts(&self) -> i32 {
        self.attempts
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
