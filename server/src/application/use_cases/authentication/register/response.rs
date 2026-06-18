use serde::Serialize;

// Registration intentionally returns no token: a verification email is sent and
// the user must confirm their address before the OAuth flow can resume.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub message: String,
}
