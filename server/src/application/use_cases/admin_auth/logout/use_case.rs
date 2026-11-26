use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::use_cases::admin_auth::logout::command::Command,
    domain::{
        errors::DomainError,
        repositories::admin_refresh_token_repository::AdminRefreshTokenRepository,
    },
    utils::oauth_crypto::sha256_hex,
};

// Server-side sign-out: revoke the presented refresh token's whole family so it
// (and any sibling rotations) can never be exchanged again. Idempotent and silent
// about whether the token existed — an unknown/already-revoked token is a no-op
// that still returns success, so logout never leaks token validity.
pub struct UseCase {
    refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
}

impl UseCase {
    pub fn new(refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>) -> Self {
        Self {
            refresh_token_repository,
        }
    }

    // Returns the admin the token belonged to, if any, for the audit log.
    pub async fn execute(&self, command: Command) -> Result<Option<Uuid>, DomainError> {
        let Some(token) = self
            .refresh_token_repository
            .find_by_token_hash(&sha256_hex(&command.refresh_token))
            .await?
        else {
            return Ok(None);
        };
        self.refresh_token_repository
            .revoke_family(token.family_id())
            .await?;
        Ok(Some(token.admin_id()))
    }
}
