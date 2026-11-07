use serde::Deserialize;
use uuid::Uuid;

// `POST /v1/auth/magic-link/redeem` body, sent by the hosted login page the emailed
// link opens. camelCase to match the SPA.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,

    // From the link: `?id=` (the token's row id) and `&token=` (the raw secret).
    pub id: Uuid,
    pub token: String,

    // The code of the session that asked for the link, read from THIS browser's
    // storage. The same-browser binding: a forwarded link, or a scanner that opens
    // it, does not have it.
    pub session_code: String,

    // Optional relying-party MFA hints, as on login.
    pub mfa_methods: Option<Vec<String>>,
    pub mfa_default_method: Option<String>,

    // Request context, injected by the handler — never taken from the JSON body.
    #[serde(skip_deserializing, default)]
    pub ip_address: Option<String>,
    #[serde(skip_deserializing, default)]
    pub user_agent: Option<String>,
}
