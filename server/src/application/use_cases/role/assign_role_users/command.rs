use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub role_id: Uuid,

    #[validate(length(min = 1, message = "At least one user is required"))]
    pub user_ids: Vec<Uuid>,
}
