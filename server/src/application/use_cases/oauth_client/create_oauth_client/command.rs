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

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // tenant_id is injected from the URL path, so skip deserializing it from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    // "public" (PKCE only, no secret) or "confidential" (server-side, gets a secret).
    #[validate(length(min = 1, message = "client_type cannot be empty"))]
    pub client_type: String,

    pub redirect_uris: Vec<String>,

    #[serde(default)]
    pub post_logout_redirect_uris: Vec<String>,

    pub allowed_scopes: Vec<String>,

    pub allowed_grant_types: Vec<String>,

    // Keep PKCE required for everyone unless explicitly opted out.
    #[serde(default = "default_true")]
    pub require_pkce: bool,

    #[serde(default = "default_true")]
    pub require_consent: bool,

    // Which tenant SMTP provider sends this client's emails. None → tenant default.
    #[serde(default)]
    pub email_provider_id: Option<Uuid>,

    // MFA policy: whether a second factor is mandatory, and which methods this
    // client accepts. Methods default to enabled so turning `mfa_required` on
    // immediately offers every factor.
    #[serde(default)]
    pub mfa_required: bool,

    #[serde(default = "default_true")]
    pub mfa_email_enabled: bool,

    #[serde(default = "default_true")]
    pub mfa_totp_enabled: bool,

    #[serde(default = "default_true")]
    pub mfa_webauthn_enabled: bool,

    // Optional per-client legal pages (OIDC tos_uri / policy_uri). Blank or
    // omitted means the auth UI falls back to the platform's /terms and /privacy.
    #[serde(default)]
    pub tos_uri: Option<String>,

    #[serde(default)]
    pub policy_uri: Option<String>,

    // Optional OIDC initiate_login_uri: the RP endpoint that starts a fresh
    // authorization request. The hosted login page offers "restart sign-in" only when
    // this is set — without it, an expired OAuth state is a dead end for the user.
    #[serde(default)]
    pub initiate_login_uri: Option<String>,

    // Optional per-client login-page template + branding. Validated by
    // validate_login_appearance before it reaches the entity.
    #[serde(default)]
    pub login_appearance: Option<LoginAppearanceInput>,

    // When true, this client authenticates ONLY through its external identity
    // providers — no password registration or sign-in. Allowed on create even with
    // no providers configured yet (they are added afterwards); update_oauth_client
    // is where the "at least one active provider" invariant is enforced.
    #[serde(default)]
    pub external_login_only: bool,

    // Which supported locales this client's hosted login/sign-up page offers,
    // and which one it defaults to absent a user choice. Validated by
    // validate_client_languages before it reaches the entity.
    #[serde(default = "default_allowed_languages")]
    pub allowed_languages: Vec<String>,

    #[serde(default = "default_language_code")]
    pub default_language: String,
}
