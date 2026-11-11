use serde::Deserialize;

// `POST /v1/auth/qr-login` body: "show me a QR code to sign in with". The same frozen
// OIDC authorization request a password login carries, and no credential at all.
// camelCase to match the hosted login SPA.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
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
