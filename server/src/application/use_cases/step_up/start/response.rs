use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

// Which proof the account gives, and what it needs to give it.
#[derive(Debug, Serialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum Response {
    // Send `current_password` with the action; nothing to start.
    Password,
    // Run a WebAuthn assertion with `options`; send it with `challengeId`.
    Passkey {
        #[serde(rename = "challengeId")]
        challenge_id: Uuid,
        options: Value,
    },
    // A 6-digit code was emailed; send it with `challengeId` before `expiresAt`.
    Email {
        #[serde(rename = "challengeId")]
        challenge_id: Uuid,
        #[serde(rename = "expiresAt")]
        expires_at: DateTime<Utc>,
    },
}
