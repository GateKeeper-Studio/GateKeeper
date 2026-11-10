use serde::Serialize;

// Shown once, to put the seed in the authenticator app (QR code or typed).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub secret_base32: String,
    pub otpauth_url: String,
}
