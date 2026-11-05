use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

fn default_true() -> bool {
    true
}

// Configures a new external identity provider for an OAuth client. camelCase to
// match the dashboard. `client_secret` is write-only: it is encrypted in the use
// case and never returned.
#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    // tenant_id and oauth_client_id are injected from the URL path.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub oauth_client_id: Uuid,

    // "google" | "github"
    #[validate(length(min = 1, message = "provider cannot be empty"))]
    pub provider: String,

    // The OAuth client id/secret issued by the external provider.
    #[validate(length(min = 1, message = "client id cannot be empty"))]
    pub client_id: String,
    #[validate(length(min = 1, message = "client secret cannot be empty"))]
    pub client_secret: String,

    #[validate(url(message = "a valid redirect URI is required"))]
    pub redirect_uri: String,

    #[serde(default = "default_true")]
    pub is_active: bool,
}
