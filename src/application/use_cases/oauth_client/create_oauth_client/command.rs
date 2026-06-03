use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

fn default_true() -> bool {
  true
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
}
