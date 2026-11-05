use chrono::{DateTime, Utc};
use serde::Serialize;

// The session code ROTATES on every refresh: the value returned here replaces the one
// the SPA held, and the previous one stops resolving immediately. The SPA must write
// this back to sessionStorage before its next call.
//
// `sessionAbsoluteExpiresAt` is unchanged by a refresh — it is the ceiling, and the UI
// uses it to know when to stop offering "keep me signed in" rather than promising an
// extension the server will refuse.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub session_code: String,
    pub session_expires_at: DateTime<Utc>,
    pub session_absolute_expires_at: DateTime<Utc>,
}
