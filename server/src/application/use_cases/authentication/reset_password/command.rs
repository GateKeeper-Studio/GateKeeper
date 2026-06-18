use serde::Deserialize;
use validator::Validate;

// Completes a "forgot password" flow: the values come from the emailed reset
// link (`passwordResetToken` + `passwordResetId`) plus the new password the user
// typed. camelCase to match the dashboard front-end.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,

    pub password_reset_token: String,
    pub password_reset_id: String,

    // OWASP-aligned minimum; argon2 hashing happens in the use case.
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
}
