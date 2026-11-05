use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // tenant_id is injected from the URL path, so skip deserializing it from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub organization_id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    pub description: Option<String>,
    pub is_active: bool,
    pub can_self_register: bool,
    pub can_self_forgot_password: bool,
    pub default_language: String,
}
