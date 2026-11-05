use uuid::Uuid;

// Built by the handler from the path and the authenticated principal — never
// deserialized from a request body, so the actor cannot be forged.
#[derive(Debug)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}
