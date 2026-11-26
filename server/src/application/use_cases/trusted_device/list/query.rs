use uuid::Uuid;

// The account's devices. Self-service passes the token's account; the admin API, the
// path's (after tenant-scope checks).
#[derive(Debug, Clone)]
pub struct Query {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}
