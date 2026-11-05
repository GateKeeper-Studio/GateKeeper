use uuid::Uuid;

// The admin id is taken from the caller's verified token, never a request input.
pub struct Command {
    pub admin_id: Uuid,
}
