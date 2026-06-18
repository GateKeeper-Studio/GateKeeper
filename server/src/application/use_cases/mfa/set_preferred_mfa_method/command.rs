use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    // Identity comes from the verified bearer token, never from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub user_id: Uuid,

    // "email" | "totp" | "webauthn", or null to fall back to the login
    // priority chain (webauthn > totp > email).
    pub method: Option<String>,
}
