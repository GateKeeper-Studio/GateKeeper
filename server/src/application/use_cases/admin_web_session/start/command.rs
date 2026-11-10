use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Command {
    pub email: String,
    pub password: String,
    // The second factor, when the admin has enabled one (see `admin_auth::login`).
    #[serde(default, alias = "mfaCode")]
    pub mfa_code: Option<String>,
}
