use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Query {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}
