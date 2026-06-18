use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConsentGrant {
    id: Uuid,
    tenant_id: Uuid,

    user_id: Uuid,   // who consented
    client_id: Uuid, // FK to oauth_clients.id

    // The scopes the user has approved for this client so far. The DB enforces
    // UNIQUE (user_id, client_id) so there is exactly one row per pairing; new
    // approvals widen this set.
    scopes: Vec<String>,

    // Revoking (from the "manage apps" screen) sets this; a revoked grant forces a
    // fresh consent prompt and is the cue to revoke live tokens for the pairing.
    revoked_at: Option<DateTime<Utc>>,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl ConsentGrant {
    pub fn new(tenant_id: Uuid, user_id: Uuid, client_id: Uuid, scopes: Vec<String>) -> Self {
        ConsentGrant {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            client_id,
            scopes,
            revoked_at: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        client_id: Uuid,
        scopes: Vec<String>,
        revoked_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        ConsentGrant {
            id,
            tenant_id,
            user_id,
            client_id,
            scopes,
            revoked_at,
            created_at,
            updated_at,
        }
    }

    // Domain rule: have ALL requested scopes already been consented (and not revoked)?
    // If true, the consent screen can be skipped entirely.
    pub fn covers(&self, requested: &[String]) -> bool {
        self.revoked_at.is_none() && requested.iter().all(|s| self.scopes.iter().any(|g| g == s))
    }

    // The scopes the user has NOT yet approved — what a delta-consent screen shows.
    pub fn missing_scopes(&self, requested: &[String]) -> Vec<String> {
        requested
            .iter()
            .filter(|s| !self.scopes.iter().any(|g| g == *s))
            .cloned()
            .collect()
    }

    // Merge newly approved scopes into the grant (idempotent union).
    pub fn grant_scopes(&mut self, newly_approved: &[String]) {
        for s in newly_approved {
            if !self.scopes.iter().any(|g| g == s) {
                self.scopes.push(s.clone());
            }
        }
        self.revoked_at = None;
        self.updated_at = Some(Utc::now());
    }

    pub fn revoke(&mut self) {
        self.revoked_at = Some(Utc::now());
        self.updated_at = Some(Utc::now());
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
    pub fn client_id(&self) -> Uuid {
        self.client_id
    }
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
    pub fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}
