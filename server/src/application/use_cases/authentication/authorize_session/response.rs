use serde::Serialize;

// Everything the SPA needs to redirect the browser back to the relying party:
// {redirectUri}?code={authorizationCode}&state={state}. `authorizationCode` is the
// RAW code — returned exactly once; only its SHA-256 hash was persisted. The values
// come from the frozen session, not from the request body.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub authorization_code: String,
    pub state: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub response_type: String,
}
