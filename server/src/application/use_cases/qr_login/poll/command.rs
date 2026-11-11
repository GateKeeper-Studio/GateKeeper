use serde::Deserialize;
use uuid::Uuid;

// `POST /v1/auth/qr-login/status` body: the browser showing a code asks what became of
// it. Authenticated by the code of the session that `start` opened, which only this
// browser holds. camelCase to match the SPA.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,
    pub request_id: Uuid,
    pub session_code: String,

    // Optional relying-party MFA hints, as on login: used when the approved sign-in
    // still needs a second factor on this browser.
    pub mfa_methods: Option<Vec<String>>,
    pub mfa_default_method: Option<String>,

    // Request context, injected by the handler — never taken from the JSON body.
    #[serde(skip_deserializing, default)]
    pub ip_address: Option<String>,
    #[serde(skip_deserializing, default)]
    pub user_agent: Option<String>,
}
