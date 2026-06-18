use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// Registers a new SMTP provider for a tenant. camelCase to match the dashboard.
// `password` is write-only: it is encrypted in the use case and never returned.
#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    // tenant_id is injected from the URL path, so skip deserializing it from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    #[validate(length(min = 1, message = "Host cannot be empty"))]
    pub host: String,

    pub port: i32,

    #[validate(length(min = 1, message = "Username cannot be empty"))]
    pub username: String,

    #[validate(length(min = 1, message = "Password cannot be empty"))]
    pub password: String,

    #[validate(email(message = "A valid from-email is required"))]
    pub from_email: String,

    pub from_name: Option<String>,

    // "starttls" | "tls" | "none"
    #[validate(length(min = 1, message = "encryption cannot be empty"))]
    pub encryption: String,

    #[serde(default)]
    pub is_default: bool,
}
