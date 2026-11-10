use uuid::Uuid;

use crate::application::use_cases::admin_web_session::identity::AdminIdentity;

// A live session, turned into what the request needs: the identity for the UI and
// a freshly minted short-lived admin access token for the bridged API call.
#[derive(Debug)]
pub struct Response {
    // The session's row id (never the cookie value). Handlers that end the admin's
    // other sessions keep this one.
    pub session_id: Uuid,
    pub admin: AdminIdentity,
    pub access_token: String,
}
