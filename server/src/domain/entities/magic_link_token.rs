use chrono::{DateTime, Utc};
use uuid::Uuid;

// One-time sign-in link mailed to the user (magic link login).
//
// The link proves control of the mailbox, but on its own it is not enough: it is
// tied to the auth session (`auth_session_id`) that the requesting browser holds the
// code for, so a forwarded copy or a link scanner, which does not have that session
// code, cannot redeem it. See `redeem_magic_link`.
//
// SECURITY: same storage rules as `PasswordResetToken`. Only SHA-256(token) is
// persisted; the raw 256-bit value lives in the emailed link only. Lookup is by the
// row `id` (carried as `&id=` in the link), then the secret is hash-compared in
// constant time, so the token is never a SQL key. A 256-bit token is not
// brute-forceable, so there is no attempts counter. Single-use (`consumed_at`) and
// short-lived (`expires_at`).
#[derive(Debug, Clone)]
pub struct MagicLinkToken {
    id: Uuid,
    tenant_id: Uuid,
    user_id: Uuid,
    auth_session_id: Uuid,

    token_hash: String,

    consumed_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl MagicLinkToken {
    pub fn new(
        tenant_id: Uuid,
        user_id: Uuid,
        auth_session_id: Uuid,
        token_hash: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        MagicLinkToken {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            auth_session_id,
            token_hash,
            consumed_at: None,
            expires_at,
            created_at: Utc::now(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        auth_session_id: Uuid,
        token_hash: String,
        consumed_at: Option<DateTime<Utc>>,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        MagicLinkToken {
            id,
            tenant_id,
            user_id,
            auth_session_id,
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
    pub fn auth_session_id(&self) -> Uuid {
        self.auth_session_id
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

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn token(expires_in: Duration) -> MagicLinkToken {
        MagicLinkToken::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            "hash".to_string(),
            Utc::now() + expires_in,
        )
    }

    #[test]
    fn a_fresh_token_is_redeemable_once() {
        let mut t = token(Duration::minutes(15));
        let now = Utc::now();
        assert!(t.is_redeemable(now));

        t.consume(now);
        assert!(t.is_consumed());
        assert!(!t.is_redeemable(now));
    }

    #[test]
    fn an_expired_token_is_not_redeemable() {
        let t = token(Duration::seconds(-1));
        assert!(t.is_expired(Utc::now()));
        assert!(!t.is_redeemable(Utc::now()));
    }
}
