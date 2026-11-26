use chrono::{DateTime, Utc};
use uuid::Uuid;

// A phone (an app the user signed in to) that holds a private key in its secure
// hardware and approves QR code sign-ins by signing them, instead of the user typing
// the password. Only the public key is stored; the private key never leaves the
// device, so a stolen access token alone cannot approve anything.
//
// `counts_as_second_factor`: whether its approvals satisfy a client's MFA policy. True
// when the registration was confirmed with the user's authenticator-app code, or the
// user had no second factor at all then (the device becomes one). Otherwise the device
// replaces the password, and the browser is still asked for a second factor.
#[derive(Debug, Clone)]
pub struct TrustedDevice {
    id: Uuid,
    tenant_id: Uuid,
    user_id: Uuid,
    name: String,
    public_key: String,
    counts_as_second_factor: bool,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

pub const MAX_NAME_LENGTH: usize = 64;
pub const MAX_PER_USER: i64 = 10;

impl TrustedDevice {
    pub fn new(
        tenant_id: Uuid,
        user_id: Uuid,
        name: String,
        public_key: String,
        counts_as_second_factor: bool,
        now: DateTime<Utc>,
    ) -> Self {
        TrustedDevice {
            id: Uuid::now_v7(),
            tenant_id,
            user_id,
            name,
            public_key,
            counts_as_second_factor,
            created_at: now,
            last_used_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        user_id: Uuid,
        name: String,
        public_key: String,
        counts_as_second_factor: bool,
        created_at: DateTime<Utc>,
        last_used_at: Option<DateTime<Utc>>,
    ) -> Self {
        TrustedDevice {
            id,
            tenant_id,
            user_id,
            name,
            public_key,
            counts_as_second_factor,
            created_at,
            last_used_at,
        }
    }

    // A display name: trimmed, 1–64 characters.
    pub fn normalize_name(name: &str) -> Option<String> {
        let name = name.trim();
        (!name.is_empty() && name.chars().count() <= MAX_NAME_LENGTH).then(|| name.to_string())
    }

    pub fn record_use(&mut self, now: DateTime<Utc>) {
        self.last_used_at = Some(now);
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn public_key(&self) -> &str {
        &self.public_key
    }
    pub fn counts_as_second_factor(&self) -> bool {
        self.counts_as_second_factor
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn last_used_at(&self) -> Option<DateTime<Utc>> {
        self.last_used_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_bounded() {
        assert_eq!(
            TrustedDevice::normalize_name("  Pixel 8 ").as_deref(),
            Some("Pixel 8")
        );
        assert!(TrustedDevice::normalize_name("   ").is_none());
        assert!(TrustedDevice::normalize_name(&"é".repeat(64)).is_some());
        assert!(TrustedDevice::normalize_name(&"é".repeat(65)).is_none());
    }
}
