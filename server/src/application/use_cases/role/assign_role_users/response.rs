use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Response {
    pub assigned: u32,
    pub user_count: u32,
}
