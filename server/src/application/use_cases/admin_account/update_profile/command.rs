use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct Command {
    // admin_id is injected from the verified token, not the request body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub admin_id: Uuid,

    #[validate(length(min = 1, max = 128, message = "Name must be 1-128 characters"))]
    pub name: String,

    #[validate(
        email(message = "Must be a valid email address"),
        length(max = 128, message = "Email must be at most 128 characters")
    )]
    pub email: String,

    // Required only when `email` changes: the email is the sign-in identifier, so
    // changing it is a step-up action.
    #[serde(default, alias = "currentPassword")]
    pub current_password: Option<String>,
}
