use uuid::Uuid;

// "Sign out everywhere" for `user_id`. Built by the handler, like revoke_user_session.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    // Request context for the audit row.
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
