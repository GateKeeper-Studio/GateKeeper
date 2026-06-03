use serde::Serialize;

// Same shape as the authorization_code grant response. The `refresh_token` here
// is the NEW rotated token (raw, returned once); the presented one is now dead.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
  pub access_token: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id_token: Option<String>,
  pub refresh_token: String,
  pub token_type: String,
  pub expires_in: i64,
  pub scope: String,
}
