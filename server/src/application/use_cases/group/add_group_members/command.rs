use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // Injected from the URL path, so skip deserializing from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub group_id: Uuid,

    #[validate(length(min = 1, message = "At least one user is required"))]
    pub user_ids: Vec<Uuid>,
}
