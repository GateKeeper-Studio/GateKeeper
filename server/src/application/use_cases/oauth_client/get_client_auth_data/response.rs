use serde::Serialize;

use crate::domain::entities::{oauth_client::OAuthClient, tenant::Tenant};

// Public auth-flow configuration for a client, consumed by the login SPA to
// decide which self-service entry points and external providers to render.
// NEVER includes secrets, redirect URIs, or any tenant-internal detail.
//
// camelCase on the wire to match the SvelteKit front-end's `ClientAuthData`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub id: String, // the public client_id
    pub name: String,
    pub can_self_sign_up: bool,
    pub can_self_forgot_pass: bool,
    // Optional per-client legal pages. None means the auth UI falls back to the
    // platform's /terms and /privacy pages.
    pub tos_uri: Option<String>,
    pub policy_uri: Option<String>,
    pub oauth_providers: Vec<OAuthProviderView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthProviderView {
    pub id: String,
    pub name: String,
}

impl Response {
    // The self-service flags are tenant-wide; the client only contributes its
    // public identity. External identity providers are not modelled yet, so the
    // list is always empty for now.
    pub fn from_entities(client: &OAuthClient, tenant: &Tenant) -> Self {
        Response {
            id: client.client_id().to_string(),
            name: client.name().to_string(),
            can_self_sign_up: tenant.can_self_register(),
            can_self_forgot_pass: tenant.can_self_forgot_password(),
            tos_uri: client.tos_uri().map(str::to_string),
            policy_uri: client.policy_uri().map(str::to_string),
            oauth_providers: Vec::new(),
        }
    }
}
