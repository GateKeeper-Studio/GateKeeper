use uuid::Uuid;

// Whose sessions. Both ids come from the server side: the token for /me, the
// tenant-scoped path for the admin API.
#[derive(Debug, Clone)]
pub struct Query {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}
