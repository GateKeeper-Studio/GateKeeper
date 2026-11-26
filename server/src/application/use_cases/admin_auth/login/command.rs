use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Command {
    pub email: String,
    pub password: String,
    // The second factor (a TOTP code or a recovery code), required when the admin
    // has enabled one. Without it the answer is `mfa_required`; send the same
    // email and password again with the code.
    #[serde(default, alias = "mfaCode")]
    pub mfa_code: Option<String>,
}
