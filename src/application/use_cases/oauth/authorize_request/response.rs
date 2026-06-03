use uuid::Uuid;

// The validated authorization request. Everything here is already checked and
// safe to carry forward into the login + consent screens (the redirect_uri is
// proven, the scopes are intersected with the client's allow-list). The handler
// freezes these values so the user cannot tamper with them via the form.
#[derive(Debug, Clone)]
pub struct Response {
  pub tenant_id: Uuid,
  pub client_internal_id: Uuid,   // oauth_clients.id (FK target)
  pub client_id: String,          // public client_id
  pub client_name: String,
  pub redirect_uri: String,
  pub scopes: Vec<String>,        // intersection of requested and allowed
  pub state: String,
  pub code_challenge: String,
  pub code_challenge_method: String,
  pub nonce: Option<String>,
  // First-party apps may set require_consent = false; the handler uses this to
  // decide whether the consent screen is ever shown.
  pub requires_consent: bool,
}
