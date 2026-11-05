use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub user_id: Uuid,
    pub custom_field_values: Option<Value>,
}
