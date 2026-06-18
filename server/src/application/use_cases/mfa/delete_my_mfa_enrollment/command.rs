use uuid::Uuid;

// Identity comes from the verified bearer token; the enrollment id from the
// URL path.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub enrollment_id: Uuid,
}
