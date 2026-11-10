use chrono::{DateTime, Utc};
use uuid::Uuid;

// Not `Serialize`: `token` is the raw link secret. The handler turns it into the
// link and returns that once.
#[derive(Debug)]
pub struct Response {
    pub invitation_id: Uuid,
    pub email: String,
    pub expires_at: DateTime<Utc>,
    pub token: String,
}
