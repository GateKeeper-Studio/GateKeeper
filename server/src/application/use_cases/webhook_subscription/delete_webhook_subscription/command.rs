use uuid::Uuid;

pub struct Command {
    pub tenant_id: Uuid,
    pub id: Uuid,
}
