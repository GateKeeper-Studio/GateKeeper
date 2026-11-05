use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// NOTE: deliberately has no `field_key` or `field_type` — both are immutable
// after creation (see CustomFieldDefinition's doc comment), so there is no way
// to change either through this Command at all, not merely "ignored if sent".
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub field_id: Uuid,

    // locale -> label text; validated in the use case (see
    // create_tenant_custom_field::Command for why this isn't a `#[validate]` attr).
    pub label: HashMap<String, String>,

    // locale -> description text. Entirely optional.
    pub description: Option<HashMap<String, String>>,

    pub options: Option<Vec<HashMap<String, String>>>,
    pub is_required: bool,
    pub display_order: i32,
    pub is_active: bool,
}
