use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

// An open invitation. The link itself is not recoverable (only its hash is kept).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationView {
    pub id: Uuid,
    pub email: String,
    pub invited_by: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub invitations: Vec<InvitationView>,
}
