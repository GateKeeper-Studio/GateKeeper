use serde::Serialize;
use uuid::Uuid;

// SECURITY: the raw secret is shown to the user exactly once, at enrollment
// time (QR code + copyable text). It is never retrievable again — only the
// encrypted copy is stored.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub enrollment_id: Uuid,
    pub secret_base32: String,
    pub otpauth_url: String,
}
