use serde::Deserialize;
use validator::Validate;

// Completes a forced "change password" flow: the user just authenticated with a
// temporary password (so login returned a one-time `changePasswordCode` and the
// `userId`), and now submits the new password. camelCase to match the dashboard
// front-end; the application is identified by its public `clientId`.
//
// `sessionCode` is the sign-in session that minted the code. The change is refused
// until that session has passed its second factor: the temporary password alone
// must never be enough to choose the account's next password.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,
    pub user_id: String,
    pub session_code: String,
    pub change_password_code: String,

    // OWASP-aligned minimum; argon2 hashing happens in the use case.
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
}
