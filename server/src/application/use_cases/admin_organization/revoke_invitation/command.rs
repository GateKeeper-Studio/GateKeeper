use uuid::Uuid;

pub struct Command {
    pub organization_id: Uuid,
    pub invitation_id: Uuid,
}
