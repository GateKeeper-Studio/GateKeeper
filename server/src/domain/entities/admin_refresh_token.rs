use chrono::{DateTime, Utc};
use uuid::Uuid;

// A refresh token for a platform-admin dashboard session. Global (tied to an
// admin account, not a tenant/client). The raw opaque token is never stored —
// only its SHA-256 hash. Each refresh rotates the token and revokes its
// predecessor; the whole rotation lineage shares a `family_id` so detecting reuse
// of an already-rotated token can revoke every sibling at once (theft response).
#[derive(Debug, Clone)]
pub struct AdminRefreshToken {
    id: Uuid,
    admin_id: Uuid,

    token_hash: String,
    family_id: Uuid,

    rotated_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,

    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl AdminRefreshToken {
    // Issues a brand-new token that starts its own rotation family.
    pub fn new(admin_id: Uuid, token_hash: String, expires_at: DateTime<Utc>) -> Self {
        AdminRefreshToken {
            id: Uuid::now_v7(),
            admin_id,
            token_hash,
            family_id: Uuid::now_v7(),
            rotated_at: None,
            revoked_at: None,
            expires_at,
            created_at: Utc::now(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        admin_id: Uuid,
        token_hash: String,
        family_id: Uuid,
        rotated_at: Option<DateTime<Utc>>,
        revoked_at: Option<DateTime<Utc>>,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        AdminRefreshToken {
            id,
            admin_id,
            token_hash,
            family_id,
            rotated_at,
            revoked_at,
            expires_at,
            created_at,
        }
    }

    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        self.revoked_at.is_none() && self.rotated_at.is_none() && now < self.expires_at
    }

    // Mark this token as rotated away and mint its successor in the same family.
    pub fn rotate(
        &mut self,
        new_token_hash: String,
        new_expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> AdminRefreshToken {
        self.rotated_at = Some(now);
        AdminRefreshToken {
            id: Uuid::now_v7(),
            admin_id: self.admin_id,
            token_hash: new_token_hash,
            family_id: self.family_id,
            rotated_at: None,
            revoked_at: None,
            expires_at: new_expires_at,
            created_at: now,
        }
    }

    pub fn revoke(&mut self, now: DateTime<Utc>) {
        self.revoked_at = Some(now);
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
    pub fn family_id(&self) -> Uuid {
        self.family_id
    }
    pub fn rotated_at(&self) -> Option<DateTime<Utc>> {
        self.rotated_at
    }
    pub fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
