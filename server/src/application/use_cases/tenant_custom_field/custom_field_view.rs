use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::custom_field_definition::CustomFieldDefinition;

// Admin-facing projection of a tenant custom field definition. Shared by the
// create/get/list/update use cases within this resource. Unlike the public
// get_client_auth_data projection, this exposes the FULL per-language map for
// every field — the admin edits every language, not just one resolved locale.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldView {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub field_key: String,
    pub label: HashMap<String, String>,
    pub description: Option<HashMap<String, String>>,
    pub field_type: String,
    pub options: Option<Vec<HashMap<String, String>>>,
    pub is_required: bool,
    pub display_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CustomFieldView {
    pub fn from_entity(def: &CustomFieldDefinition) -> Self {
        CustomFieldView {
            id: def.id(),
            tenant_id: def.tenant_id(),
            field_key: def.field_key().to_string(),
            label: def.label().clone(),
            description: def.description().cloned(),
            field_type: def.field_type().as_str().to_string(),
            options: def.options().map(|o| o.to_vec()),
            is_required: def.is_required(),
            display_order: def.display_order(),
            is_active: def.is_active(),
            created_at: def.created_at(),
            updated_at: def.updated_at(),
        }
    }
}
