use std::sync::Arc;

use crate::{
    application::use_cases::admin_web_session::end::command::Command,
    domain::{
        errors::DomainError, repositories::admin_web_session_repository::AdminWebSessionRepository,
    },
    utils::oauth_crypto::sha256_hex,
};

// Console sign-out: deletes the session row, so the cookie is dead even if a copy
// leaked. Idempotent and silent about whether the session existed.
//
// There is no refresh-token family to revoke: a console session never had one, and
// the access tokens the bridge minted expire on their own within minutes.
pub struct UseCase {
    session_repository: Arc<dyn AdminWebSessionRepository>,
}

impl UseCase {
    pub fn new(session_repository: Arc<dyn AdminWebSessionRepository>) -> Self {
        Self { session_repository }
    }

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        self.session_repository
            .delete_by_token_hash(&sha256_hex(&command.session_token))
            .await
    }
}
