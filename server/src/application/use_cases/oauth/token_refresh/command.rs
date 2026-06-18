use uuid::Uuid;

// `POST /oauth/token` with grant_type=refresh_token.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub refresh_token: String,
}
