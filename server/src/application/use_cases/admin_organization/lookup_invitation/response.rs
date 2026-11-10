use chrono::{DateTime, Utc};
use serde::Serialize;

// What the invitation page shows before the invitee picks a password.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub email: String,
    pub organization_name: String,
    pub expires_at: DateTime<Utc>,
}
