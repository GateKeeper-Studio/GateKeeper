use serde::Serialize;

use crate::application::use_cases::oauth_client::client_view::ClientView;

// The created client, plus the raw client secret for confidential clients —
// returned EXACTLY ONCE here and never retrievable again (only its Argon2 hash
// is stored). Public clients get `client_secret: None`.
#[derive(Debug, Serialize)]
pub struct Response {
  #[serde(flatten)]
  pub client: ClientView,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub client_secret: Option<String>,
}
