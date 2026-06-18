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
}
