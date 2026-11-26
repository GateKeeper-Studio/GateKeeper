use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

// What the admin learns: where the temporary password went and until when it works.
// Never the password itself.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub user_id: Uuid,
    pub email: String,
    pub temporary_password_expires_at: DateTime<Utc>,
}
