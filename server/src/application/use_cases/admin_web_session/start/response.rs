use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::use_cases::admin_web_session::identity::AdminIdentity;

// Deliberately not `Serialize`: `session_token` is the raw cookie value, which only
// ever travels in a Set-Cookie header. The handler returns `admin` alone as JSON.
#[derive(Debug)]
pub struct Response {
    pub session_token: String,
    // For audit attribution.
    pub admin_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub admin: AdminIdentity,
}

#[derive(Debug)]
pub enum Outcome {
    Started(Response),
    // The password is right; the console asks for the code and signs in again.
    SecondFactorRequired { admin_id: Uuid },
}
