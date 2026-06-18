use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Query {
    pub tenant_id: Uuid,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}
