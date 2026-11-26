use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    // The welcome email's temporary password works until then (once). Never the
    // password itself.
    pub temporary_password_expires_at: DateTime<Utc>,
    // False when the account was created but the email could not be delivered: an
    // admin password reset then sends a new one.
    pub temporary_password_emailed: bool,
}
