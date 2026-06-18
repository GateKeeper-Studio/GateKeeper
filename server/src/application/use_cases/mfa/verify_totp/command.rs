use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,
    pub session_code: String,
    pub mfa_id: Option<Uuid>,
    pub code: String,
}
