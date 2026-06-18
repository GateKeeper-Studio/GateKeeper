use serde::Deserialize;
use validator::Validate;

// "Forgot password" request. camelCase to match the dashboard front-end. The
// application is identified by its public `clientId`; the tenant is derived from
// it server-side.
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,

    #[validate(email(message = "A valid email is required"))]
    pub email: String,
}
