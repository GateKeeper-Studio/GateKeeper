use serde::Deserialize;

// `POST /v1/auth/session/refresh` body. camelCase to match the login SPA.
//
// Note what is NOT here: no email, no password, no protocol parameters. Extending a
// session must not be able to change anything about it — the frozen /authorize
// request and the bound user are read back from the session itself. The session code
// is the only credential, and the client id exists solely to derive the tenant and
// re-check that the session was opened for this very client.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    // The opaque session token most recently issued for this flow (raw value).
    pub session_code: String,
    // The application's public id.
    pub client_id: String,
}
