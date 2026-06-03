use uuid::Uuid;

// The parsed `GET /oauth/authorize` query string. This use case only validates
// it — no writes happen here, because at this point the user is not yet
// authenticated. See `authorize_complete` for the write side.
#[derive(Debug, Clone)]
pub struct Query {
  pub tenant_id: Uuid,
  pub client_id: String,
  pub redirect_uri: String,
  pub response_type: String,
  pub scope: String,
  pub state: String,
  pub code_challenge: String,
  pub code_challenge_method: String,
  pub nonce: Option<String>,
}
