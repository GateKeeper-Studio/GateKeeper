use serde::Serialize;

// The OAuth token response. `id_token` is only present when the `openid` scope
// was granted (OIDC). `refresh_token` is the RAW opaque token — returned once;
// only its hash is stored.
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
