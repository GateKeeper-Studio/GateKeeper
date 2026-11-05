use serde::Deserialize;
use validator::Validate;

// `POST /v1/auth/login` body. camelCase to match the SvelteKit login SPA. The
// application is identified by its public `clientId`; the tenant is derived from
// it server-side. The OIDC parameters are the validated authorization request the
// SPA carried over from `GET /authorize`; we freeze them into the auth session so
// the second step (`/v1/auth/authorize`) cannot be tampered with.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    #[validate(email(message = "A valid email is required"))]
    pub email: String,
    pub password: String,

    pub client_id: String,

    // ── Frozen OIDC authorization request ───────────────────────────────────────
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub nonce: Option<String>,

    // Optional relying-party UX hints. The IdP still enforces the stored client
    // MFA policy and the user's enrollments; invalid or unavailable hints are
    // ignored by the use case.
    pub mfa_methods: Option<Vec<String>>,
    pub mfa_default_method: Option<String>,

    // Request context for the Risk Engine, injected by the handler from
    // X-Forwarded-For / X-Real-IP / User-Agent — never taken from the JSON body.
    #[serde(skip_deserializing, default)]
    pub ip_address: Option<String>,
    #[serde(skip_deserializing, default)]
    pub user_agent: Option<String>,
}
