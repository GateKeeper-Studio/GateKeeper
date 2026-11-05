use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::email_verification_token::EmailVerificationToken, errors::DomainError,
};

#[async_trait]
pub trait EmailVerificationTokenRepository: Send + Sync {
    // The 6-digit code is not unique across users, so confirmation is scoped to the
    // user (resolved by email first): this returns the user's most recent
    // unconsumed code, if any.
    async fn find_active_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<EmailVerificationToken>, DomainError>;
    async fn add(&self, token: &EmailVerificationToken) -> Result<(), DomainError>;
    // Persists redemption (consumed_at) and the wrong-attempt counter.
    async fn update(&self, token: &EmailVerificationToken) -> Result<(), DomainError>;

    // Retention: hard-delete rows already past `expires_at`. Called by the
    // background purge; idempotent, so concurrent runs are harmless.
    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError>;
}
