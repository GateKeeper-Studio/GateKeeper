use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub mfa_id: String,
    // The creation options for navigator.credentials.create(). The ceremony
    // state stays server-side in the challenge row, never in this response.
    pub options: Value,
}
