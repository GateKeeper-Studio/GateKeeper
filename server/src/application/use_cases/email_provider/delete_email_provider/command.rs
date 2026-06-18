use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub id: Uuid,
}
