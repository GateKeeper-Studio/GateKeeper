use serde::{Deserialize, Serialize};
use uuid::Uuid;

// The full ordered set of a tenant's field ids — index becomes display_order.
// Must be exactly a permutation of the tenant's existing field ids (checked in
// the use case); no partial/PATCH-style reorder is supported.
#[derive(Debug, Serialize, Deserialize)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    pub field_ids: Vec<Uuid>,
}
