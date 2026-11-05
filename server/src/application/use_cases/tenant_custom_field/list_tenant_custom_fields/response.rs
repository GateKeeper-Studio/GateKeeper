use serde::Serialize;

use crate::application::use_cases::tenant_custom_field::custom_field_view::CustomFieldView;

// Deliberately unpaged: field count is hard-capped per tenant (see
// create_tenant_custom_field::MAX_FIELDS_PER_TENANT), so the admin management
// view always fetches the whole (small) list at once.
#[derive(Debug, Serialize)]
pub struct Response {
    pub fields: Vec<CustomFieldView>,
}
