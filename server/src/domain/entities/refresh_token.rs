use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RefreshToken {
    id: Uuid,
    tenant_id: Uuid,

    token_hash: String, // SHA-256 of the opaque token; never store raw
    client_id: Uuid,
    user_id: Uuid,
    scopes: Vec<String>,

    // Rotation chain: each refresh issues a new token and revokes the old one.
    // A whole family shares a lineage so we can revoke ALL of them if reuse of a
    // already-rotated token is detected (token theft).
    family_id: Uuid,
    // Set when this token is rotated away. Presenting a token that has a
    // rotated_at is the reuse-detection trigger.
    rotated_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,

    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl RefreshToken {
    // Issues a brand-new token that starts its own rotation family.
    pub fn new(
        tenant_id: Uuid,
        token_hash: String,
        client_id: Uuid,
        user_id: Uuid,
        scopes: Vec<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        RefreshToken {
            id: Uuid::now_v7(),
            tenant_id,
            token_hash,
            client_id,
            user_id,
            scopes,
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
        tenant_id: Uuid,
        token_hash: String,
        client_id: Uuid,
        user_id: Uuid,
        scopes: Vec<String>,
        family_id: Uuid,
        rotated_at: Option<DateTime<Utc>>,
        revoked_at: Option<DateTime<Utc>>,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        RefreshToken {
            id,
            tenant_id,
            token_hash,
            client_id,
            user_id,
            scopes,
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
    ) -> RefreshToken {
        self.rotated_at = Some(now);
        RefreshToken {
            id: Uuid::now_v7(),
            tenant_id: self.tenant_id,
            token_hash: new_token_hash,
            client_id: self.client_id,
            user_id: self.user_id,
            scopes: self.scopes.clone(),
            family_id: self.family_id,
            rotated_at: None,
            revoked_at: None,
            expires_at: new_expires_at,
            created_at: now,
        }
    }

    // Issue another token in the SAME family WITHOUT mutating this one. Used for the
    // rotation reuse-leeway: a just-rotated token presented again within the grace
    // window yields a fresh sibling instead of tripping reuse detection. Because the
    // presented token is left untouched, its `rotated_at` — and therefore the leeway
    // window — never slides forward on repeated presentation.
    pub fn reissue_within_family(
        &self,
        new_token_hash: String,
        new_expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> RefreshToken {
        RefreshToken {
            id: Uuid::now_v7(),
            tenant_id: self.tenant_id,
            token_hash: new_token_hash,
            client_id: self.client_id,
            user_id: self.user_id,
            scopes: self.scopes.clone(),
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
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn token_hash(&self) -> &str {
        &self.token_hash
    }
    pub fn client_id(&self) -> Uuid {
        self.client_id
    }
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn scopes(&self) -> &[String] {
        &self.scopes
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
