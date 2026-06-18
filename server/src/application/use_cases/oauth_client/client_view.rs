use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::oauth_client::OAuthClient;

// The public projection of an OAuth client. NEVER includes the client secret hash
// (or the secret) — the raw secret is shown exactly once, at creation, in the
// create use case's own response.
#[derive(Debug, Clone, Serialize)]
pub struct ClientView {
    pub id: Uuid,
    pub client_id: String,
    pub name: String,
    pub client_type: String,
    pub redirect_uris: Vec<String>,
    pub post_logout_redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub allowed_grant_types: Vec<String>,
    pub require_pkce: bool,
    pub require_consent: bool,
    pub email_provider_id: Option<Uuid>,
    pub tos_uri: Option<String>,
    pub policy_uri: Option<String>,
    pub mfa_required: bool,
    pub mfa_email_enabled: bool,
    pub mfa_totp_enabled: bool,
    pub mfa_webauthn_enabled: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl ClientView {
    pub fn from_entity(client: &OAuthClient) -> Self {
        ClientView {
            id: client.id(),
            client_id: client.client_id().to_string(),
            name: client.name().to_string(),
            client_type: client.client_type().as_str().to_string(),
            redirect_uris: client.redirect_uris().to_vec(),
            post_logout_redirect_uris: client.post_logout_redirect_uris().to_vec(),
            allowed_scopes: client.allowed_scopes().to_vec(),
            allowed_grant_types: client.allowed_grant_types().to_vec(),
            require_pkce: client.require_pkce(),
            require_consent: client.require_consent(),
            email_provider_id: client.email_provider_id(),
            tos_uri: client.tos_uri().map(str::to_string),
            policy_uri: client.policy_uri().map(str::to_string),
            mfa_required: client.mfa_policy().required,
            mfa_email_enabled: client.mfa_policy().email_enabled,
            mfa_totp_enabled: client.mfa_policy().totp_enabled,
            mfa_webauthn_enabled: client.mfa_policy().webauthn_enabled,
            is_active: client.is_active(),
            created_at: client.created_at().to_rfc3339(),
            updated_at: client.updated_at().map(|t| t.to_rfc3339()),
        }
    }
}
