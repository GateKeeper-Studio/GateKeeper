use uuid::Uuid;

// Records that `user_id` accepted `document_version` of `document_type`. The
// handler injects tenant_id/user_id (from path or principal) and the client IP.
#[derive(Debug)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub document_type: String,
    pub document_version: String,
    pub ip_address: Option<String>,
}
