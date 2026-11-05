use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::{
        mfa_challenge::{MfaChallenge, MfaChallengePurpose},
        mfa_method::MfaMethod,
    },
    errors::DomainError,
};

#[async_trait]
pub trait MfaChallengeRepository: Send + Sync {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<MfaChallenge>, DomainError>;
    // The most recent unconsumed challenge of this method bound to the session
    // (email OTP lookups are scoped to the session, never to the code).
    async fn find_active_by_session(
        &self,
        auth_session_id: Uuid,
        method: &MfaMethod,
    ) -> Result<Option<MfaChallenge>, DomainError>;

    async fn add(&self, challenge: &MfaChallenge) -> Result<(), DomainError>;
    // Persists redemption (consumed_at) and the wrong-attempt counter.
    async fn update(&self, challenge: &MfaChallenge) -> Result<(), DomainError>;
    // Burn any still-active challenges before minting a replacement (resend /
    // method switch), so only one live code exists per user+purpose+method.
    async fn invalidate_active_by_user(
        &self,
        user_id: Uuid,
        purpose: &MfaChallengePurpose,
        method: &MfaMethod,
    ) -> Result<(), DomainError>;

    // Retention: hard-delete rows already past `expires_at`. Called by the
    // background purge; idempotent, so concurrent runs are harmless.
    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError>;
}
