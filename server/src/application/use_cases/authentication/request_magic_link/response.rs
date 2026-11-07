use chrono::{DateTime, Utc};
use serde::Serialize;

// Identical for every email, whether or not it has an account (see the use case).
//
// `sessionCode` is the RAW code of the session waiting for the click. The hosted login
// keeps it in this browser's storage: redeeming the emailed link requires it, which is
// what makes the link work only in the browser that asked for it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub session_code: String,
    // When the waiting session — and with it the link — dies.
    pub session_expires_at: DateTime<Utc>,
    pub session_absolute_expires_at: DateTime<Utc>,
}
