use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

// A browser session of the platform-admin console, used when this server serves the
// dashboard itself. The browser holds a random opaque id in an httpOnly cookie; only
// its SHA-256 hash is stored here. The session carries no token: each `/api` request
// gets a freshly minted short-lived admin access token (see the cookie bridge).
//
// Two clocks end it: an absolute lifetime fixed at sign-in, and an idle timeout
// measured from the last request.
#[derive(Debug, Clone)]
pub struct AdminWebSession {
    id: Uuid,
    admin_id: Uuid,
    token_hash: String,
    created_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl AdminWebSession {
    pub fn new(admin_id: Uuid, token_hash: String, now: DateTime<Utc>, lifetime: Duration) -> Self {
        AdminWebSession {
            id: Uuid::now_v7(),
            admin_id,
            token_hash,
            created_at: now,
            last_seen_at: now,
            expires_at: now + lifetime,
        }
    }

    pub fn reconstruct(
        id: Uuid,
        admin_id: Uuid,
        token_hash: String,
        created_at: DateTime<Utc>,
        last_seen_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        AdminWebSession {
            id,
            admin_id,
            token_hash,
            created_at,
            last_seen_at,
            expires_at,
        }
    }

    // Usable right now: inside its absolute lifetime and not idle for too long.
    pub fn is_live(&self, now: DateTime<Utc>, idle_timeout: Duration) -> bool {
        now < self.expires_at && now - self.last_seen_at < idle_timeout
    }

    // Whether `last_seen_at` is stale enough to be worth a write. Touching on every
    // request would turn each read of the console into a database write.
    pub fn needs_touch(&self, now: DateTime<Utc>, granularity: Duration) -> bool {
        now - self.last_seen_at >= granularity
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn admin_id(&self) -> Uuid {
        self.admin_id
    }
    pub fn token_hash(&self) -> &str {
        &self.token_hash
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn last_seen_at(&self) -> DateTime<Utc> {
        self.last_seen_at
    }
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_ends_at_its_absolute_lifetime_and_when_idle() {
        let now = Utc::now();
        let session =
            AdminWebSession::new(Uuid::now_v7(), "hash".to_string(), now, Duration::hours(12));
        let idle = Duration::hours(2);

        assert!(session.is_live(now, idle));
        assert!(session.is_live(now + Duration::minutes(119), idle));
        assert!(!session.is_live(now + Duration::hours(2), idle), "idle");
        assert!(
            !session.is_live(now + Duration::hours(12), Duration::hours(24)),
            "absolute lifetime"
        );
    }

    #[test]
    fn touching_is_throttled() {
        let now = Utc::now();
        let session = AdminWebSession::new(Uuid::now_v7(), "h".into(), now, Duration::hours(1));
        assert!(!session.needs_touch(now + Duration::seconds(30), Duration::minutes(1)));
        assert!(session.needs_touch(now + Duration::minutes(1), Duration::minutes(1)));
    }
}
