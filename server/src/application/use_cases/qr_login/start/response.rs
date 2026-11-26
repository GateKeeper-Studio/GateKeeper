use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

// The new code, and the browser's half of it. `requestId` + `token` go into the QR
// code (the handler builds the URL); `sessionCode` stays in this browser and is what
// it polls with. Each raw secret is returned once; only hashes are stored.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub request_id: Uuid,
    pub token: String,
    pub session_code: String,
    // When the code can no longer be scanned. The browser asks for a new one then.
    pub expires_at: DateTime<Utc>,
    pub poll_interval_seconds: i64,
    pub session_expires_at: DateTime<Utc>,
    pub session_absolute_expires_at: DateTime<Utc>,
}
