use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub organization_id: Uuid,
    // The colleague acted on (from the path).
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub admin_id: Uuid,
    #[serde(skip_deserializing)]
    pub active: bool,
    #[serde(default, alias = "currentPassword")]
    pub current_password: Option<String>,
}
