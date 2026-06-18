use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    // Identity comes from the verified bearer token, never from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub user_id: Uuid,

    pub mfa_id: Uuid,
    // User-facing label for the new passkey, e.g. "YubiKey 5C".
    pub name: Option<String>,
    // The attestation response from navigator.credentials.create().
    pub attestation: Value,
}
