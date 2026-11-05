use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub field_id: Uuid,
}
