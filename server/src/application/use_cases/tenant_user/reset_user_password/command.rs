use uuid::Uuid;

// Built by the handler from the path, the principal and the request context; there
// is no body. Nothing here is a password: the server makes the temporary one.
#[derive(Debug)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
