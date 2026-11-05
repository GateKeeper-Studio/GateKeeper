use std::collections::HashMap;

use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::oauth_client::{LoginAppearance, OAuthClient};

// Serialized view of a client's login-page appearance. camelCase on the wire so
// the dashboard can read it back and POST the same shape unchanged. Shared by the
// admin ClientView and the public get_client_auth_data projection.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginAppearanceView {
    pub layout: String,
    pub hero_position: String,
    pub logo_uri: Option<String>,
    pub background_image_uri: Option<String>,
    pub heading: Option<HashMap<String, String>>,
    pub subtext: Option<HashMap<String, String>>,
    pub primary_color: Option<String>,
    pub accent_color: Option<String>,
    pub border_radius: Option<String>,
    pub font_family: Option<String>,
}

impl LoginAppearanceView {
    pub fn from_domain(a: &LoginAppearance) -> Self {
        LoginAppearanceView {
            layout: a.layout.as_str().to_string(),
            hero_position: a.hero_position.as_str().to_string(),
            logo_uri: a.logo_uri.clone(),
            background_image_uri: a.background_image_uri.clone(),
            heading: a.heading.clone(),
            subtext: a.subtext.clone(),
            primary_color: a.primary_color.clone(),
            accent_color: a.accent_color.clone(),
            border_radius: a.border_radius.clone(),
            font_family: a.font_family.map(|f| f.as_str().to_string()),
        }
    }
}

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
    pub initiate_login_uri: Option<String>,
    pub login_appearance: Option<LoginAppearanceView>,
    // When true, this client's users authenticate only through its external identity
    // providers — every password-credential path is refused for it.
    pub external_login_only: bool,
    pub allowed_languages: Vec<String>,
    pub default_language: String,
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
            initiate_login_uri: client.initiate_login_uri().map(str::to_string),
            login_appearance: client
                .login_appearance()
                .map(LoginAppearanceView::from_domain),
            external_login_only: client.external_login_only(),
            allowed_languages: client.allowed_languages().to_vec(),
            default_language: client.default_language().to_string(),
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
