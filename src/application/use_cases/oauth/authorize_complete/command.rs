use uuid::Uuid;

// Submitted after the user authenticated (reuse the `login` use case to verify
// the password) and answered the consent screen. The protocol fields are carried
// over from the validated `authorize_request` so they cannot be tampered with.
#[derive(Debug, Clone)]
pub struct Command {
  pub tenant_id: Uuid,
  pub client_id: String,
  pub user_id: Uuid,
  pub redirect_uri: String,
  pub scopes: Vec<String>,
  pub state: String,
  pub code_challenge: String,
  pub code_challenge_method: String,
  pub nonce: Option<String>,
  // The user clicked "Allow". When false and consent is still missing, the use
  // case returns ConsentRequired so the handler re-renders the consent screen.
  pub user_approved: bool,
}
