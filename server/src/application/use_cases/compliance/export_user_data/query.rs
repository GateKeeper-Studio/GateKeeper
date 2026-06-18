use uuid::Uuid;

// Identifies the data subject whose data is exported. `tenant_id` is enforced as a
// boundary: a user from another tenant is treated as not found.
#[derive(Debug)]
pub struct Query {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}
