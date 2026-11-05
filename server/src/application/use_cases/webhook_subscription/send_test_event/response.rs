use serde::Serialize;

// The raw result of one synchronous attempt, so the admin sees exactly what their
// endpoint returned instead of having to go read the delivery log.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub success: bool,
    pub status: Option<u16>,
    pub error: Option<String>,
}
