use uuid::Uuid;

// DELETE /me/trusted-devices/{id} and DELETE /tenants/{t}/users/{u}/trusted-devices/{id}.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
