use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

fn default_true() -> bool {
    true
}

// Updates the editable details of a client. Identity fields (client_id, secret,
// client_type) are deliberately not updatable through this path.
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    pub redirect_uris: Vec<String>,

    #[serde(default)]
    pub post_logout_redirect_uris: Vec<String>,

    pub allowed_scopes: Vec<String>,

    pub allowed_grant_types: Vec<String>,

    #[serde(default = "default_true")]
    pub require_pkce: bool,

    #[serde(default = "default_true")]
    pub require_consent: bool,

    #[serde(default = "default_true")]
    pub is_active: bool,

    // Which tenant SMTP provider sends this client's emails. None → tenant default.
    #[serde(default)]
    pub email_provider_id: Option<Uuid>,

    // MFA policy (see create_oauth_client::Command).
    #[serde(default)]
    pub mfa_required: bool,

    #[serde(default = "default_true")]
    pub mfa_email_enabled: bool,

    #[serde(default = "default_true")]
    pub mfa_totp_enabled: bool,

    #[serde(default = "default_true")]
    pub mfa_webauthn_enabled: bool,

    // Optional per-client legal pages (see create_oauth_client::Command).
    #[serde(default)]
    pub tos_uri: Option<String>,

    #[serde(default)]
    pub policy_uri: Option<String>,
}
