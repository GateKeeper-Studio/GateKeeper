use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // tenant_id is injected from the URL path, so skip deserializing it from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    // Immutable identifier used as the JSON key under which submitted values are
    // stored (`^[a-z][a-z0-9_]{0,63}$`, checked in the use case — a small fixed
    // pattern, not a caller-suppliable one, so there's no ReDoS surface here).
    #[validate(length(min = 1, max = 64, message = "Field key must be 1-64 characters"))]
    pub field_key: String,

    // locale -> label text. Must include at least the tenant's own
    // default_language (checked in the use case, via validate_localized_text —
    // `validator`'s derive can't express a per-key length/membership check
    // against a HashMap, so this map is entirely hand-validated there).
    pub label: HashMap<String, String>,

    // locale -> description text. Entirely optional (sparse is fine).
    pub description: Option<HashMap<String, String>>,

    // One of "text" | "number" | "boolean" | "date" | "select" — parsed via
    // CustomFieldType::parse in the use case.
    pub field_type: String,

    // Only meaningful (and required) when field_type is "select". An ORDERED
    // list of choices, each a locale -> text map (see CustomFieldDefinition's
    // doc comment for why options are shaped this way).
    pub options: Option<Vec<HashMap<String, String>>>,

    pub is_required: bool,
    pub display_order: i32,
}
