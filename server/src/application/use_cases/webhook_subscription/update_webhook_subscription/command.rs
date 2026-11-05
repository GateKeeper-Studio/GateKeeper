use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    #[validate(length(min = 1, message = "URL cannot be empty"))]
    pub url: String,

    #[serde(default)]
    pub event_types: Vec<String>,

    pub is_active: bool,

    // Opt-in secret rotation. Absent/false keeps the current secret, so an ordinary
    // edit (renaming, toggling active) cannot silently break every consumer's
    // signature verification.
    #[serde(default)]
    pub rotate_secret: bool,
}
