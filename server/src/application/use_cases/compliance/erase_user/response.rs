use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Response {
    pub user_id: Uuid,
    pub anonymized: bool,
}
