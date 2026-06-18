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

    #[validate(length(
        min = 1,
        max = 128,
        message = "Name must be between 1 and 128 characters"
    ))]
    pub name: String,

    #[validate(length(max = 500, message = "Description cannot exceed 500 characters"))]
    pub description: Option<String>,
}
