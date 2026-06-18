use uuid::Uuid;

// No body: identity comes from the verified bearer token (AuthPrincipal).
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}
