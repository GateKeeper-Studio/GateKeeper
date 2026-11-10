use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // organization_id is injected from the URL path, not the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub organization_id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    pub description: Option<String>,

    // Suspending an organization stops every one of its admins from signing in, so
    // its own admins cannot do it (they would lock themselves out with no way back).
    // Only the system may change it; omitted keeps the current value.
    #[serde(default)]
    pub is_active: Option<bool>,
}
