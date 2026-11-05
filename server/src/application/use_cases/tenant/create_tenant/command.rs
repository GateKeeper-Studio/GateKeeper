use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// No `password_hash_secret`: the server generates each tenant's pepper. An old
// client that still sends the field has it ignored.
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    #[serde(skip)]
    pub organization_id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    pub description: Option<String>,

    pub can_self_register: bool,
    pub can_self_forgot_password: bool,
}
