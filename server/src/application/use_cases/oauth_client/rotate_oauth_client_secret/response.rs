use serde::Serialize;

use crate::application::use_cases::oauth_client::client_view::ClientView;

// The updated client, plus the freshly generated raw client secret — returned
// EXACTLY ONCE here and never retrievable again (only its Argon2 hash is stored).
// Rotation only applies to confidential clients, so the secret is always present.
#[derive(Debug, Serialize)]
pub struct Response {
    #[serde(flatten)]
    pub client: ClientView,
    pub client_secret: String,
}
