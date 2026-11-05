use uuid::Uuid;

// Built in the handler from the path param; the list endpoint has no body.
pub struct Query {
    pub tenant_id: Uuid,
}
