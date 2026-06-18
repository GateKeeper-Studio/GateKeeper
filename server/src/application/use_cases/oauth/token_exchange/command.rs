use uuid::Uuid;

// `POST /oauth/token` with grant_type=authorization_code. Arrives as
// application/x-www-form-urlencoded. `client_secret` is present only for
// confidential clients (it may also come via HTTP Basic auth at the handler).
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub code: String,
    pub redirect_uri: String,
    pub code_verifier: String,
}
