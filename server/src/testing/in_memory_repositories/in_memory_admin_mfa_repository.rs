use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::admin_mfa::AdminMfa, errors::DomainError,
    repositories::admin_mfa_repository::AdminMfaRepository,
};

// In-memory admin second-factor store for use-case tests, with the same conditional
// semantics as the database statements.
#[derive(Default)]
pub struct InMemoryAdminMfaRepository {
    pub factors: Mutex<HashMap<Uuid, AdminMfa>>,
}

impl InMemoryAdminMfaRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

fn with_changes(
    mfa: &AdminMfa,
    confirmed_at: Option<DateTime<Utc>>,
    last_step: Option<i64>,
    hashes: Vec<String>,
    at: DateTime<Utc>,
) -> AdminMfa {
    AdminMfa::reconstruct(
        mfa.admin_id(),
        mfa.totp_secret_enc().to_string(),
        confirmed_at,
        last_step,
        hashes,
        mfa.created_at(),
        Some(at),
    )
}

#[async_trait]
impl AdminMfaRepository for InMemoryAdminMfaRepository {
    async fn find(&self, admin_id: Uuid) -> Result<Option<AdminMfa>, DomainError> {
        Ok(self.factors.lock().unwrap().get(&admin_id).cloned())
    }

    async fn save_pending(&self, mfa: &AdminMfa) -> Result<bool, DomainError> {
        let mut factors = self.factors.lock().unwrap();
        if factors
            .get(&mfa.admin_id())
            .is_some_and(AdminMfa::is_enabled)
        {
            return Ok(false);
        }
        factors.insert(mfa.admin_id(), mfa.clone());
        Ok(true)
    }

    async fn confirm(
        &self,
        admin_id: Uuid,
        step: i64,
        recovery_code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut factors = self.factors.lock().unwrap();
        match factors.get_mut(&admin_id) {
            Some(mfa) if !mfa.is_enabled() => {
                *mfa = with_changes(mfa, Some(at), Some(step), recovery_code_hashes.to_vec(), at);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn record_totp_step(
        &self,
        admin_id: Uuid,
        step: i64,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut factors = self.factors.lock().unwrap();
        match factors.get_mut(&admin_id) {
            Some(mfa) if mfa.is_enabled() && mfa.accepts_step(step) => {
                let hashes = mfa.recovery_code_hashes().to_vec();
                *mfa = with_changes(mfa, mfa.totp_confirmed_at(), Some(step), hashes, at);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn consume_recovery_code(
        &self,
        admin_id: Uuid,
        code_hash: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut factors = self.factors.lock().unwrap();
        match factors.get_mut(&admin_id) {
            Some(mfa)
                if mfa.is_enabled()
                    && mfa.recovery_code_hashes().iter().any(|h| h == code_hash) =>
            {
                let hashes = mfa
                    .recovery_code_hashes()
                    .iter()
                    .filter(|h| *h != code_hash)
                    .cloned()
                    .collect();
                *mfa = with_changes(
                    mfa,
                    mfa.totp_confirmed_at(),
                    mfa.totp_last_step(),
                    hashes,
                    at,
                );
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn replace_recovery_codes(
        &self,
        admin_id: Uuid,
        recovery_code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut factors = self.factors.lock().unwrap();
        match factors.get_mut(&admin_id) {
            Some(mfa) if mfa.is_enabled() => {
                *mfa = with_changes(
                    mfa,
                    mfa.totp_confirmed_at(),
                    mfa.totp_last_step(),
                    recovery_code_hashes.to_vec(),
                    at,
                );
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn delete(&self, admin_id: Uuid) -> Result<(), DomainError> {
        self.factors.lock().unwrap().remove(&admin_id);
        Ok(())
    }
}
