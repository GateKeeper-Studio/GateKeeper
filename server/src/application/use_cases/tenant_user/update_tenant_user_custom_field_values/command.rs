use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

// Split off from update_tenant_user_by_id deliberately (same reasoning as
// Tenant's update_tenant_appearance vs. update_tenant_by_id): that use case
// does a full UserProfile replace on every call, so folding custom field
// values into it would mean any name-only edit silently wipes previously
// saved values. This Command/use case touches ONLY custom_field_values.
//
// No #[validate(...)] on `custom_field_values` — it's a raw payload validated
// manually by validate_custom_field_values in the use case, matching how
// update_tenant_appearance::Command handles its own JSON blob.
#[derive(Debug, Deserialize, Serialize)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub user_id: Uuid,

    #[serde(default)]
    pub custom_field_values: Option<Value>,
}
