use uuid::Uuid;

// `POST /oauth/token` with grant_type=refresh_token.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub refresh_token: String,
    // Injected server-side from the request context (never client-supplied), so the
    // refresh can be geo-enriched and risk-scored like an interactive login.
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
