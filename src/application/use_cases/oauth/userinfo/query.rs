use uuid::Uuid;

// `GET /oauth/userinfo`. The access token arrives in the Authorization: Bearer
// header; the handler extracts it and the tenant context.
#[derive(Debug, Clone)]
pub struct Query {
  pub tenant_id: Uuid,
  pub access_token: String,
}
