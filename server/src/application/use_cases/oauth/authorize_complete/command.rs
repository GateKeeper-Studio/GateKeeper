use uuid::Uuid;

// Submitted when the user answers the consent screen. The user is authenticated
// HERE, from their own credentials — never from a caller-supplied id, which would
// let anyone mint a code for any account. The protocol fields are carried over from
// the validated `authorize_request` so they cannot be tampered with.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub client_id: String,
    // The end-user's own credentials. We resolve and verify these to derive the
    // authenticated user id; the request body never dictates *which* user logs in.
    pub email: String,
    pub password: String,
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
