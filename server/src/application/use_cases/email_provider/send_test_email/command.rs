use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

// Sends a one-off test email through an existing provider so an admin can verify
// its SMTP credentials. `tenant_id` and `id` are injected from the URL path; only
// the recipient comes from the body.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub id: Uuid,

    #[validate(email(message = "A valid recipient email is required"))]
    pub to: String,
}
