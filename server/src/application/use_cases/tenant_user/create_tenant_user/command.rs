use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // tenant_id is injected from the URL path, so skip deserializing it from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    #[validate(length(min = 1, message = "Email cannot be empty"))]
    pub email: String,

    #[validate(length(min = 1, message = "Display name cannot be empty"))]
    pub display_name: Option<String>,

    #[validate(length(min = 1, message = "First name cannot be empty"))]
    pub first_name: String,

    #[validate(length(min = 1, message = "Last name cannot be empty"))]
    pub last_name: String,

    pub is_email_verified: bool,

    // No longer accepted: the server makes the temporary password and emails it, so
    // the admin never knows it. Kept only to refuse a caller that still sends one,
    // instead of silently ignoring a password it believes it set.
    #[serde(default, skip_serializing)]
    pub temporary_password: Option<String>,
}
