use uuid::Uuid;

// `POST /me/trusted-devices`. Built by the handler: the account comes from the
// verified access token, never from the body.
#[derive(Debug, Clone)]
pub struct Command {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    // Shown to the user ("Pixel 8"). 1–64 characters.
    pub name: String,
    // ES256 public key: SPKI DER or a SEC1 point, base64 (see utils/device_key.rs).
    pub public_key: String,
    // Signature of `shared::registration_message` with the matching private key.
    pub signature: String,
    // The account's current password: a token alone may not add a way in.
    pub current_password: Option<String>,
    // Optional: a current code from the user's authenticator app. With it, the
    // device's approvals count as a second factor.
    pub mfa_code: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
