use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub organization_id: Uuid,
    #[validate(
        email(message = "Must be a valid email address"),
        length(max = 128, message = "Email must be at most 128 characters")
    )]
    pub email: String,
    #[serde(default, alias = "currentPassword")]
    pub current_password: Option<String>,
}
