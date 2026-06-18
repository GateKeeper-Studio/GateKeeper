use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    // Identity comes from the verified bearer token; the enrollment id from
    // the URL path.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub user_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub enrollment_id: Uuid,

    // New user-facing label; null/empty clears it.
    pub name: Option<String>,
}
