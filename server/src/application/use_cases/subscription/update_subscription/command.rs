use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // organization_id is injected from the URL path, not the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub organization_id: Uuid,

    // Allows moving the subscription to a different plan.
    pub plan_id: Uuid,

    pub is_active: bool,
}
