use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{domain::entities::trusted_device::TrustedDevice, utils::device_key};

// What a device signs. Plain text, one value per line, so an app builds it with string
// concatenation and nothing else. Each starts with a fixed label, so a signature made
// for one purpose can never be presented for another.

// At registration, proving the app holds the private key: the user id (the `sub` of
// the app's ID token) and the public key exactly as sent.
pub fn registration_message(user_id: Uuid, public_key_as_sent: &str) -> String {
    format!("gatekeeper.trusted-device.register.v1\n{user_id}\n{public_key_as_sent}")
}

// To approve a QR code: the request id (from the code) and the claim code (from the
// scan). The claim code is random and single-use, so a signature cannot be replayed.
pub fn qr_approval_message(request_id: Uuid, claim_code: &str) -> String {
    format!("gatekeeper.qr-login.approve.v1\n{request_id}\n{claim_code}")
}

// A device as its user and their admins see it. Never the key.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedDeviceView {
    pub id: Uuid,
    pub name: String,
    pub algorithm: &'static str,
    pub counts_as_second_factor: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl From<&TrustedDevice> for TrustedDeviceView {
    fn from(device: &TrustedDevice) -> Self {
        TrustedDeviceView {
            id: device.id(),
            name: device.name().to_string(),
            algorithm: device_key::ALGORITHM,
            counts_as_second_factor: device.counts_as_second_factor(),
            created_at: device.created_at(),
            last_used_at: device.last_used_at(),
        }
    }
}
