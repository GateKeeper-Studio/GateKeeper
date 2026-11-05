use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // tenant_id is injected from the URL path, never taken from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    // Validated by validate_webhook_url before it reaches the entity — this is an
    // SSRF boundary, not a formatting check.
    #[validate(length(min = 1, message = "URL cannot be empty"))]
    pub url: String,

    // Which event types to receive. Omitted or empty means ALL of them.
    #[serde(default)]
    pub event_types: Vec<String>,

    #[serde(default = "default_true")]
    pub is_active: bool,
}
