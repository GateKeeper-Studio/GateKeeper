use uuid::Uuid;

// Identity comes from the verified bearer token (AuthPrincipal), never from
// the request body.
#[derive(Debug, Clone)]
pub struct Query {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}
