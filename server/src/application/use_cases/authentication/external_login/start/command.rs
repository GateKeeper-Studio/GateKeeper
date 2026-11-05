use serde::Deserialize;
use uuid::Uuid;

// Starts a "Sign in with <provider>" flow. The login SPA sends the chosen
// provider's id plus the original OAuth client request parameters (frozen here so
// they survive the round-trip to the external provider). camelCase matches the
// front-end `oauthProviderLoginApi` body; `provider_name` comes from the URL path.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    #[serde(skip_deserializing, default)]
    pub provider_name: String,

    pub oauth_provider_id: Uuid,

    pub client_code_challenge: String,
    pub client_code_challenge_method: String,
    pub client_redirect_uri: String,
    pub client_state: String,
    pub client_response_type: String,
    pub client_scope: String,
    pub client_nonce: Option<String>,
}
