use uuid::Uuid;

// For the audit log; the handler answers with the email to sign in with.
#[derive(Debug)]
pub struct Response {
    pub admin_id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
}
