use uuid::Uuid;

// The admin id is taken from the caller's verified token, never a request input.
pub struct Query {
    pub admin_id: Uuid,
}
