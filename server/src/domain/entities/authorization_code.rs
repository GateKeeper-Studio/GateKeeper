use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthorizationCode {
    id: Uuid,
    tenant_id: Uuid,

    // SECURITY: never store the raw code. Store SHA-256(code). The raw value is
    // returned to the browser exactly once (step 6) and never persisted.
    code_hash: String,

    client_id: String, // FK to oauth_clients.id
    user_id: Uuid,     // who authentitcated

    redirect_uri: String, // Must equal the one sent again at /token. Exact match only, no wildcards.
    scopes: Vec<String>,

    // PKCE: stored at /authorize, verified at /token.
    code_challenge: String,
    code_challenge_method: String, // only "S256" — reject "plain"

    nonce: String, // OIDC: echoed into the id_token to stop replay
    // OIDC auth_time / acr / amr can live here too if you support them.

    // Single-use enforcement: set once redeemed. A second redemption is an attack
    // signal — see "authorization code replay" in the security section.
    consumed_at: Option<DateTime<Utc>>,

    expires_at: DateTime<Utc>, // ~30–60 seconds from creation
    created_at: DateTime<Utc>,
}

impl AuthorizationCode {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: Uuid,
        code_hash: String,
        client_id: String,
        user_id: Uuid,
        redirect_uri: String,
        scopes: Vec<String>,
        code_challenge: String,
        code_challenge_method: String,
        nonce: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        AuthorizationCode {
            id: Uuid::now_v7(),
            tenant_id,
            code_hash,
            client_id,
            user_id,
            redirect_uri,
            scopes,
            code_challenge,
            code_challenge_method,
            nonce,
            consumed_at: None,
            expires_at,
            created_at: Utc::now(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        code_hash: String,
        client_id: String,
        user_id: Uuid,
        redirect_uri: String,
        scopes: Vec<String>,
        code_challenge: String,
        code_challenge_method: String,
        nonce: String,
        consumed_at: Option<DateTime<Utc>>,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        AuthorizationCode {
            id,
            tenant_id,
            code_hash,
            client_id,
            user_id,
            redirect_uri,
            scopes,
            code_challenge,
            code_challenge_method,
            nonce,
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
    pub fn code_hash(&self) -> &str {
        &self.code_hash
    }
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }
    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }
    pub fn code_challenge(&self) -> &str {
        &self.code_challenge
    }
    pub fn code_challenge_method(&self) -> &str {
        &self.code_challenge_method
    }
    pub fn nonce(&self) -> &str {
        &self.nonce
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
