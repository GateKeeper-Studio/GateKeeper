use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::password_change_token::PasswordChangeToken, errors::DomainError};

#[async_trait]
pub trait PasswordChangeTokenRepository: Send + Sync {
    // The change-password request carries the user id, so the token is resolved per
    // user: this returns the user's most recent unconsumed token, if any.
    async fn find_active_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<PasswordChangeToken>, DomainError>;
    async fn add(&self, token: &PasswordChangeToken) -> Result<(), DomainError>;
    // Persists redemption (consumed_at).
    async fn update(&self, token: &PasswordChangeToken) -> Result<(), DomainError>;
    // Burns every still-live token for the user so a fresh login never leaves more
    // than one live change code at a time.
    async fn invalidate_active_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError>;
}
