use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Query {
    pub tenant_id: Uuid,
    pub id: Uuid, // internal oauth_clients.id
}
