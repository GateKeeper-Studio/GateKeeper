use serde::Serialize;

// The recovery codes, shown once. Only their hashes are kept.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub recovery_codes: Vec<String>,
}
