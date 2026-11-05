use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct Command {
    // admin_id is injected from the verified token, not the request body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub admin_id: Uuid,

    // Re-authentication: the current password must be presented to change it, so a
    // hijacked-but-unlocked session can't silently set a new one.
    pub current_password: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
}
