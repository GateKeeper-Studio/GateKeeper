use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub group_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub member_count: u32,
    pub created_at: String,
}
