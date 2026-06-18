use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::password_reset_token::PasswordResetToken, errors::DomainError};

#[async_trait]
pub trait PasswordResetTokenRepository: Send + Sync {
    // Reset links carry their row id (`&id=`), so redemption looks the token up by
    // id and then hash-compares the secret — the high-entropy token is never a
    // lookup key.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PasswordResetToken>, DomainError>;
    async fn add(&self, token: &PasswordResetToken) -> Result<(), DomainError>;
    // Persists redemption (consumed_at).
    async fn update(&self, token: &PasswordResetToken) -> Result<(), DomainError>;
    // Burns every still-live reset token for the user so only one link is ever
    // active at a time: issuing a fresh link invalidates any older ones.
    async fn invalidate_active_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError>;
}
