use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

fn default_true() -> bool {
    true
}

// Updates an SMTP provider. `password` is optional: when omitted (or empty) the
// stored password is kept; when present it is re-encrypted and replaced.
#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    #[validate(length(min = 1, message = "Host cannot be empty"))]
    pub host: String,

    pub port: i32,

    #[validate(length(min = 1, message = "Username cannot be empty"))]
    pub username: String,

    #[serde(default)]
    pub password: Option<String>,

    #[validate(email(message = "A valid from-email is required"))]
    pub from_email: String,

    pub from_name: Option<String>,

    #[validate(length(min = 1, message = "encryption cannot be empty"))]
    pub encryption: String,

    #[serde(default)]
    pub is_default: bool,

    #[serde(default = "default_true")]
    pub is_active: bool,
}
