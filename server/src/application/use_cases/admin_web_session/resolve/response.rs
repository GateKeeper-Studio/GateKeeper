use crate::application::use_cases::admin_web_session::identity::AdminIdentity;

// A live session, turned into what the request needs: the identity for the UI and
// a freshly minted short-lived admin access token for the bridged API call.
#[derive(Debug)]
pub struct Response {
    pub admin: AdminIdentity,
    pub access_token: String,
}
