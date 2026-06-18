use serde::Deserialize;

// Sent by the confirm-email screen's "resend" button. The application is
// identified by its public `clientId`; the tenant is derived server-side.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub client_id: String,
    pub email: String,
}
