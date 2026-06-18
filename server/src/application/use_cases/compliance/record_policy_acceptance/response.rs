use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Response {
    pub document_type: String,
    pub document_version: String,
    pub accepted_at: String,
}
