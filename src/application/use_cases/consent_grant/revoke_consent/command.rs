use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Command {
  pub tenant_id: Uuid,
  pub user_id: Uuid,
  pub client_id: Uuid, // internal oauth_clients.id
}
