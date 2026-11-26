use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub admin_id: Uuid,
    #[serde(default, alias = "currentPassword")]
    pub current_password: Option<String>,
    // A current TOTP code or an unused recovery code. Not needed to cancel an
    // enrollment that was never confirmed.
    #[serde(default)]
    pub code: Option<String>,
}
