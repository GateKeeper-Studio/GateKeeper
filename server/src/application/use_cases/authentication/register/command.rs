use serde::Deserialize;
use validator::Validate;

// Self-registration request. The wire format is camelCase to match the dashboard
// front-end. The application is identified by its public `clientId` — the tenant
// is derived from it server-side. OIDC params are no longer needed here: a
// 6-digit code is emailed and the dashboard re-supplies the OIDC context at
// confirm time.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,

    #[validate(email(message = "A valid email is required"))]
    pub email: String,

    // OWASP-aligned minimum; argon2 hashing happens in the use case.
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    pub display_name: Option<String>,

    #[validate(length(min = 1, message = "First name cannot be empty"))]
    pub first_name: String,

    #[validate(length(min = 1, message = "Last name cannot be empty"))]
    pub last_name: String,
}
