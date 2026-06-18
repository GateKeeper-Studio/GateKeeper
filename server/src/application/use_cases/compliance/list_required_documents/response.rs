use serde::Serialize;

// For each legal document currently in force: whether THIS user has accepted the
// required version. The UI uses `acceptance_required` to gate access / show a prompt.
#[derive(Debug, Serialize)]
pub struct Response {
    pub document_type: String,
    pub required_version: String,
    pub accepted_version: Option<String>,
    pub acceptance_required: bool,
}
