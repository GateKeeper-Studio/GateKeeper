use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::application::use_cases::oauth_client::validation::LoginAppearanceInput;

fn default_true() -> bool {
    true
}

fn default_allowed_languages() -> Vec<String> {
    vec!["en".to_string()]
}

fn default_language_code() -> String {
    "en".to_string()
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

    // Optional OIDC initiate_login_uri (see create_oauth_client::Command).
    #[serde(default)]
    pub initiate_login_uri: Option<String>,

    // Optional per-client login-page template + branding. Validated by
    // validate_login_appearance before it ever reaches the entity; null clears it.
    #[serde(default)]
    pub login_appearance: Option<LoginAppearanceInput>,

    // When true, this client authenticates ONLY through its external identity
    // providers (see create_oauth_client::Command). The use case refuses to turn it
    // on unless the client already has at least one ACTIVE provider.
    #[serde(default)]
    pub external_login_only: bool,

    // Magic link login ("email me a sign-in link"). This is a full replace like every
    // other field here: a caller that omits it turns the feature off.
    #[serde(default)]
    pub magic_link_enabled: bool,

    // Which supported locales this client's hosted login/sign-up page offers,
    // and which one it defaults to absent a user choice. Validated by
    // validate_client_languages before it reaches the entity.
    #[serde(default = "default_allowed_languages")]
    pub allowed_languages: Vec<String>,

    #[serde(default = "default_language_code")]
    pub default_language: String,
}
