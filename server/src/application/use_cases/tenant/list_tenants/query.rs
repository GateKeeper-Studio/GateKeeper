use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Query {
    #[serde(skip)]
    pub organization_id: Uuid,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}
