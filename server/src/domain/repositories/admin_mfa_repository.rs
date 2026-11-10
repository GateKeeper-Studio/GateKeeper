use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::admin_mfa::AdminMfa, errors::DomainError};

// Persistence contract for platform admins' second factors. Every state change is a
// single conditional statement, so two concurrent requests cannot both use the same
// TOTP step or the same recovery code, and an enabled factor is never overwritten by
// a new enrollment.
#[async_trait]
pub trait AdminMfaRepository: Send + Sync {
    async fn find(&self, admin_id: Uuid) -> Result<Option<AdminMfa>, DomainError>;

    // Stores a pending enrollment, replacing an earlier pending one. Returns false,
    // writing nothing, when the admin already has an enabled factor.
    async fn save_pending(&self, mfa: &AdminMfa) -> Result<bool, DomainError>;

    // Puts a pending enrollment in force, recording the step of the code that
    // confirmed it and the hashes of a fresh set of recovery codes. Returns false
    // when there is no pending enrollment.
    async fn confirm(
        &self,
        admin_id: Uuid,
        step: i64,
        recovery_code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // Accepts a TOTP step for an enabled factor if it is newer than the last one.
    // Returns false for a replay.
    async fn record_totp_step(
        &self,
        admin_id: Uuid,
        step: i64,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // Uses up one recovery code of an enabled factor. Returns false when the hash
    // is not among the unused codes.
    async fn consume_recovery_code(
        &self,
        admin_id: Uuid,
        code_hash: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // Replaces every recovery code of an enabled factor. Returns false when the
    // factor is not enabled.
    async fn replace_recovery_codes(
        &self,
        admin_id: Uuid,
        recovery_code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // Removes the factor and its recovery codes. Idempotent.
    async fn delete(&self, admin_id: Uuid) -> Result<(), DomainError>;
}
