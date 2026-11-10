use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    // Sign-in asks for a code.
    pub totp_enabled: bool,
    pub enabled_at: Option<DateTime<Utc>>,
    pub recovery_codes_remaining: usize,
}
