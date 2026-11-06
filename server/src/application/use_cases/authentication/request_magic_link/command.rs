use serde::Deserialize;
use validator::Validate;

// `POST /v1/auth/magic-link` body: "email me a sign-in link". The same frozen OIDC
// authorization request a password login carries, minus the password. camelCase to
// match the hosted login SPA.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    #[validate(email(message = "A valid email is required"))]
    pub email: String,

    pub client_id: String,

    // ── Frozen OIDC authorization request ───────────────────────────────────────
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub nonce: Option<String>,

    // Request context, injected by the handler — never taken from the JSON body.
    #[serde(skip_deserializing, default)]
    pub ip_address: Option<String>,
    #[serde(skip_deserializing, default)]
    pub user_agent: Option<String>,
}
