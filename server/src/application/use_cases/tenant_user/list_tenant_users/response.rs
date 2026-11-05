use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    // Login methods attached to this account, so the UI can render badges and
    // make linked native + social identities visible as one user.
    pub has_password: bool,
    pub providers: Vec<String>,
}
