use chrono::{DateTime, Utc};
use uuid::Uuid;

// A platform admin's second factor: an authenticator app (TOTP) and the single-use
// recovery codes that stand in for it when the phone is lost.
//
// The row exists from the moment enrollment starts, but the factor is in force only
// once `totp_confirmed_at` is set: an abandoned enrollment never blocks sign-in.
// The seed is sealed by the application (`SecretPurpose::AdminTotpSeed`, bound to
// the admin id); recovery codes are kept only as SHA-256 hashes.
#[derive(Debug, Clone)]
pub struct AdminMfa {
    admin_id: Uuid,
    totp_secret_enc: String,
    totp_confirmed_at: Option<DateTime<Utc>>,
    // The last accepted TOTP time step. A code for this step or an earlier one is a
    // replay and is refused.
    totp_last_step: Option<i64>,
    recovery_code_hashes: Vec<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl AdminMfa {
    // A pending enrollment: the seed exists, the factor is not in force yet.
    pub fn pending(admin_id: Uuid, totp_secret_enc: String, now: DateTime<Utc>) -> Self {
        AdminMfa {
            admin_id,
            totp_secret_enc,
            totp_confirmed_at: None,
            totp_last_step: None,
            recovery_code_hashes: Vec::new(),
            created_at: now,
            updated_at: None,
        }
    }

    pub fn reconstruct(
        admin_id: Uuid,
        totp_secret_enc: String,
        totp_confirmed_at: Option<DateTime<Utc>>,
        totp_last_step: Option<i64>,
        recovery_code_hashes: Vec<String>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        AdminMfa {
            admin_id,
            totp_secret_enc,
            totp_confirmed_at,
            totp_last_step,
            recovery_code_hashes,
            created_at,
            updated_at,
        }
    }

    // Whether sign-in requires the second factor.
    pub fn is_enabled(&self) -> bool {
        self.totp_confirmed_at.is_some()
    }

    // Whether `step` is newer than every step accepted so far.
    pub fn accepts_step(&self, step: i64) -> bool {
        self.totp_last_step.is_none_or(|last| step > last)
    }

    pub fn admin_id(&self) -> Uuid {
        self.admin_id
    }
    pub fn totp_secret_enc(&self) -> &str {
        &self.totp_secret_enc
    }
    pub fn totp_confirmed_at(&self) -> Option<DateTime<Utc>> {
        self.totp_confirmed_at
    }
    pub fn totp_last_step(&self) -> Option<i64> {
        self.totp_last_step
    }
    pub fn recovery_code_hashes(&self) -> &[String] {
        &self.recovery_code_hashes
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pending_enrollment_is_not_in_force_and_steps_only_move_forward() {
        let now = Utc::now();
        let pending = AdminMfa::pending(Uuid::now_v7(), "sealed".into(), now);
        assert!(!pending.is_enabled());
        assert!(pending.accepts_step(1));

        let enabled = AdminMfa::reconstruct(
            pending.admin_id(),
            "sealed".into(),
            Some(now),
            Some(100),
            vec![],
            now,
            None,
        );
        assert!(enabled.is_enabled());
        assert!(!enabled.accepts_step(100), "replay of the same step");
        assert!(!enabled.accepts_step(99));
        assert!(enabled.accepts_step(101));
    }
}
