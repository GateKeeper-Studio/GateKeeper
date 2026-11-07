use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,
    pub session_code: String,
    pub mfa_id: Uuid,
    pub assertion_data: Value,

    // Request context for the Risk Engine and the audit trail of a passwordless
    // login, injected by the handler from `RequestContext` — never from the body.
    #[serde(skip_deserializing, default)]
    pub ip_address: Option<String>,
    #[serde(skip_deserializing, default)]
    pub user_agent: Option<String>,
}
