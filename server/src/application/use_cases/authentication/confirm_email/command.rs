use serde::Deserialize;

// Submitted when the user clicks the verification link. Carries the single-use
// token plus the OIDC parameters resumed from the link so the authorization-code
// flow can complete without a password re-entry. camelCase to match the
// front-end; the application is identified by its public `clientId`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,
    pub token: String,
    pub email: String,

    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub response_type: String,
    pub nonce: Option<String>,
}
