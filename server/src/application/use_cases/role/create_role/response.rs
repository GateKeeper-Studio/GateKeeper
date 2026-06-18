use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub role_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub user_count: u32,
    pub created_at: String,
    pub updated_at: Option<String>,
}
