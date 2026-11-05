use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::external_identity_provider::ExternalProviderKind;

// Transient record bridging the two legs of an external login. The start
// endpoint mints a random opaque `state`, stores only its SHA-256 hash, and
// freezes the originating OAuth client request here so the provider callback can
// replay it (untampered) to mint our own authorization code. Single-use and
// short-lived: deleted as soon as it is consumed.
#[derive(Debug, Clone)]
pub struct ExternalLoginFlow {
    id: Uuid,
    // SHA-256 hex of the opaque state echoed back by the provider — the lookup key.
    state_hash: String,
    external_provider_id: Uuid,
    oauth_client_id: Uuid,
    tenant_id: Uuid,
    provider: ExternalProviderKind,

    // The frozen, server-trusted client authorization request, echoed back to the
    // client app once the external login completes.
    client_redirect_uri: String,
    client_response_type: String,
    client_scope: String,
    client_state: String,
    client_nonce: Option<String>,
    client_code_challenge: String,
    client_code_challenge_method: String,

    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl ExternalLoginFlow {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state_hash: String,
        external_provider_id: Uuid,
        oauth_client_id: Uuid,
        tenant_id: Uuid,
        provider: ExternalProviderKind,
        client_redirect_uri: String,
        client_response_type: String,
        client_scope: String,
        client_state: String,
        client_nonce: Option<String>,
        client_code_challenge: String,
        client_code_challenge_method: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        ExternalLoginFlow {
            id: Uuid::now_v7(),
            state_hash,
            external_provider_id,
            oauth_client_id,
            tenant_id,
            provider,
            client_redirect_uri,
            client_response_type,
            client_scope,
            client_state,
            client_nonce,
            client_code_challenge,
            client_code_challenge_method,
            expires_at,
            created_at: Utc::now(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        state_hash: String,
        external_provider_id: Uuid,
        oauth_client_id: Uuid,
        tenant_id: Uuid,
        provider: ExternalProviderKind,
        client_redirect_uri: String,
        client_response_type: String,
        client_scope: String,
        client_state: String,
        client_nonce: Option<String>,
        client_code_challenge: String,
        client_code_challenge_method: String,
        expires_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
    ) -> Self {
        ExternalLoginFlow {
            id,
            state_hash,
            external_provider_id,
            oauth_client_id,
            tenant_id,
            provider,
            client_redirect_uri,
            client_response_type,
            client_scope,
            client_state,
            client_nonce,
            client_code_challenge,
            client_code_challenge_method,
            expires_at,
            created_at,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn state_hash(&self) -> &str {
        &self.state_hash
    }
    pub fn external_provider_id(&self) -> Uuid {
        self.external_provider_id
    }
    pub fn oauth_client_id(&self) -> Uuid {
        self.oauth_client_id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn provider(&self) -> ExternalProviderKind {
        self.provider
    }
    pub fn client_redirect_uri(&self) -> &str {
        &self.client_redirect_uri
    }
    pub fn client_response_type(&self) -> &str {
        &self.client_response_type
    }
    pub fn client_scope(&self) -> &str {
        &self.client_scope
    }
    pub fn client_state(&self) -> &str {
        &self.client_state
    }
    pub fn client_nonce(&self) -> Option<&str> {
        self.client_nonce.as_deref()
    }
    pub fn client_code_challenge(&self) -> &str {
        &self.client_code_challenge
    }
    pub fn client_code_challenge_method(&self) -> &str {
        &self.client_code_challenge_method
    }
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
