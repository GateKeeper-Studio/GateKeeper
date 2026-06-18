use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // The organization that will own this subscription (at most one per org).
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub organization_id: Uuid,

    // The plan this subscription is bound to.
    pub plan_id: Uuid,
}
