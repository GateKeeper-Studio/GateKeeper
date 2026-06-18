use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,
    pub session_code: String,
    pub mfa_id: Uuid,
    pub assertion_data: Value,
}
