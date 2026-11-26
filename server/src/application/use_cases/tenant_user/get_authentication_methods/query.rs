use uuid::Uuid;

pub struct Query {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}
