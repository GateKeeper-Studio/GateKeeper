use chrono::{DateTime, Utc};
use serde::Serialize;

// What the scanning device shows before the user approves: which app, which browser,
// roughly where, and when. Plus the claim code, which approving or denying requires.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub claim_code: String,
    // The app's PUBLIC client_id and display name.
    pub client_id: String,
    pub client_name: String,
    pub requested_at: DateTime<Utc>,
    // The decision deadline.
    pub expires_at: DateTime<Utc>,
    pub requester: RequesterView,
    // Hosted page only: the hosted page signs the user in on this device before it
    // approves, and that sign-in needs a redirect URI and scopes of this client. They
    // are the browser's own (both public: registered on the client). Never used to
    // issue anything on this device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sign_in: Option<SignInView>,
}

// No IP address: the location is enough to recognise one's own computer, and the
// scanner may not be the person who asked.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequesterView {
    pub user_agent: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignInView {
    pub redirect_uri: String,
    pub scope: String,
}
