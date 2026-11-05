use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct Command {
    // admin_id is injected from the verified token, not the request body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub admin_id: Uuid,

    #[validate(length(min = 1, max = 128, message = "Name cannot be empty"))]
    pub name: String,

    #[validate(email(message = "Must be a valid email address"))]
    pub email: String,
}
