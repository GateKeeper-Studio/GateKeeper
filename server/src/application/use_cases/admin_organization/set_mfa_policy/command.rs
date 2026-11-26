use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub organization_id: Uuid,
    pub required: bool,
    #[serde(default, alias = "currentPassword")]
    pub current_password: Option<String>,
}
