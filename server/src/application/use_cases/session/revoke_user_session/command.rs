use uuid::Uuid;

// End one of `user_id`'s sessions. Built by the handler: the ids come from the token
// (/me) or the tenant-scoped path (admin), never from a body.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    // The session id from the list (the refresh-token family id).
    pub session_id: Uuid,
    // Request context for the audit row.
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
