use uuid::Uuid;

#[derive(Debug)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}
