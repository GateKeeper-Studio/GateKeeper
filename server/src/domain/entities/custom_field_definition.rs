use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

// The kinds of custom input a tenant admin can define. Deliberately closed —
// no free-form validation pattern (e.g. regex) is ever accepted from an admin,
// to avoid ReDoS and to keep every value renderable as plain text (no HTML/
// rich-text type exists, so the dashboard never needs to render one of these
// values as anything but an escaped text node).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomFieldType {
    Text,
    Number,
    Boolean,
    Date,
    Select,
}

impl CustomFieldType {
    // Persisted as a lowercase discriminator string (tenant_custom_fields.field_type).
    pub fn as_str(&self) -> &'static str {
        match self {
            CustomFieldType::Text => "text",
            CustomFieldType::Number => "number",
            CustomFieldType::Boolean => "boolean",
            CustomFieldType::Date => "date",
            CustomFieldType::Select => "select",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "text" => Some(CustomFieldType::Text),
            "number" => Some(CustomFieldType::Number),
            "boolean" => Some(CustomFieldType::Boolean),
            "date" => Some(CustomFieldType::Date),
            "select" => Some(CustomFieldType::Select),
            _ => None,
        }
    }
}

// A tenant-defined input field used to collect additional data from that
// tenant's users (at sign-up and/or later profile edits). `field_key` and
// `field_type` are immutable once created — changing either could strand
// previously-submitted values in a shape that no longer matches the
// definition, so both are simply absent from `update()`.
#[derive(Debug, Clone)]
pub struct CustomFieldDefinition {
    id: Uuid,
    tenant_id: Uuid,
    field_key: String,
    // locale -> label text; must contain at least the owning tenant's
    // default_language (enforced in the application layer, not here).
    label: HashMap<String, String>,
    // locale -> description text; sparse is fine (no locale is required).
    description: Option<HashMap<String, String>>,
    field_type: CustomFieldType,
    // Only meaningful when field_type is Select. An ORDERED list of choices —
    // each entry is one choice's per-language text (locale -> text). Modeling
    // it this way (rather than HashMap<locale, Vec<String>>) keeps every
    // language's translations index-aligned by construction: there is only
    // one array to reorder/add/remove from, not N parallel ones that could
    // drift out of sync.
    options: Option<Vec<HashMap<String, String>>>,
    is_required: bool,
    display_order: i32,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CustomFieldDefinition {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: Uuid,
        field_key: String,
        label: HashMap<String, String>,
        description: Option<HashMap<String, String>>,
        field_type: CustomFieldType,
        options: Option<Vec<HashMap<String, String>>>,
        is_required: bool,
        display_order: i32,
    ) -> Self {
        let now = Utc::now();
        CustomFieldDefinition {
            id: Uuid::now_v7(),
            tenant_id,
            field_key,
            label,
            description,
            field_type,
            options,
            is_required,
            display_order,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        field_key: String,
        label: HashMap<String, String>,
        description: Option<HashMap<String, String>>,
        field_type: CustomFieldType,
        options: Option<Vec<HashMap<String, String>>>,
        is_required: bool,
        display_order: i32,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        CustomFieldDefinition {
            id,
            tenant_id,
            field_key,
            label,
            description,
            field_type,
            options,
            is_required,
            display_order,
            is_active,
            created_at,
            updated_at,
        }
    }

    // Mutates every editable attribute. `field_key` and `field_type` are
    // deliberately not parameters here — they can't be changed after creation.
    pub fn update(
        &mut self,
        label: HashMap<String, String>,
        description: Option<HashMap<String, String>>,
        options: Option<Vec<HashMap<String, String>>>,
        is_required: bool,
        display_order: i32,
        is_active: bool,
    ) {
        self.label = label;
        self.description = description;
        self.options = options;
        self.is_required = is_required;
        self.display_order = display_order;
        self.is_active = is_active;
        self.updated_at = Utc::now();
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn field_key(&self) -> &str {
        &self.field_key
    }
    pub fn label(&self) -> &HashMap<String, String> {
        &self.label
    }
    pub fn description(&self) -> Option<&HashMap<String, String>> {
        self.description.as_ref()
    }
    pub fn field_type(&self) -> CustomFieldType {
        self.field_type
    }
    pub fn options(&self) -> Option<&[HashMap<String, String>]> {
        self.options.as_deref()
    }
    pub fn is_required(&self) -> bool {
        self.is_required
    }
    pub fn display_order(&self) -> i32 {
        self.display_order
    }
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
