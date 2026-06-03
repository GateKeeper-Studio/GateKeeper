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
}
