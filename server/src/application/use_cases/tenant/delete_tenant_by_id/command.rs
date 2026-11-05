use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    pub tenant_id: Uuid,
    #[serde(skip)]
    pub organization_id: Uuid,
}
