use serde::Deserialize;

// `POST /v1/auth/authorize` body. camelCase to match the login SPA. The protocol
// parameters the SPA re-sends are deliberately NOT trusted here — the request was
// frozen into the auth session at login time, so we read it back from the session
// and ignore any re-posted values that could differ. We keep only the fields we
// actually rely on; serde drops the rest.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    // The opaque session token returned by `/v1/auth/login` (its raw value).
    pub session_code: String,
    // The application's public id, used to derive the tenant and re-check the
    // session was opened for this very client.
    pub client_id: String,
    // The email the SPA believes is logged in; cross-checked against the session's
    // bound user as defence in depth.
    pub email: String,
}
