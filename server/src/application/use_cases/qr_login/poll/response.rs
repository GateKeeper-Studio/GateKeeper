use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::application::use_cases::authentication::login::response::Response as LoginResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    // Not scanned yet.
    Pending,
    // A device scanned it and is deciding (the user may be signing in there).
    Scanned,
    // The device approved: this browser is signed in (`login`).
    Approved,
    Denied,
    // Nobody scanned it in time, or the decision ran out. Ask for a new code.
    Expired,
}

// `login` (only when approved) has the shape a password login answers with, so the
// SPA continues exactly as after a password: MFA screen, forced password change, or
// POST /v1/auth/authorize. Its `sessionCode` replaces the one used to poll.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    // Who approved: the account this browser is now signed in to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<LoginResponse>,
}

impl Response {
    pub fn waiting(status: Status, expires_at: DateTime<Utc>) -> Self {
        Response {
            status,
            expires_at: Some(expires_at),
            email: None,
            login: None,
        }
    }

    pub fn ended(status: Status) -> Self {
        Response {
            status,
            expires_at: None,
            email: None,
            login: None,
        }
    }
}
