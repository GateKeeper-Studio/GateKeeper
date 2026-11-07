use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::password_hasher::PasswordHasher,
        shared::admin_credentials::verify_admin_credentials,
        use_cases::admin_web_session::{
            identity::AdminIdentity,
            session_lifetime,
            start::{command::Command, response::Response},
        },
    },
    domain::{
        entities::admin_web_session::AdminWebSession,
        errors::DomainError,
        repositories::{
            admin_user_repository::AdminUserRepository,
            admin_web_session_repository::AdminWebSessionRepository,
        },
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// Signs a platform admin in to the console: verifies the password exactly like
// `POST /admin/login` (same generic failure, same timing), then opens a browser
// session. Unlike `/admin/login` it hands out no bearer token and no refresh token:
// the caller gets an opaque session id for an httpOnly cookie, and only its hash is
// stored.
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    session_repository: Arc<dyn AdminWebSessionRepository>,
    password_pepper: String,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl UseCase {
    pub fn new(
        admin_repository: Arc<dyn AdminUserRepository>,
        session_repository: Arc<dyn AdminWebSessionRepository>,
        password_pepper: String,
        password_hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            admin_repository,
            session_repository,
            password_pepper,
            password_hasher,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let admin = verify_admin_credentials(
            self.admin_repository.as_ref(),
            self.password_hasher.as_ref(),
            &self.password_pepper,
            &command.email,
            &command.password,
        )
        .await?;

        // 256 random bits; the browser keeps it, the database keeps its SHA-256.
        let session_token = generate_opaque_token();
        let session = AdminWebSession::new(
            admin.id(),
            sha256_hex(&session_token),
            Utc::now(),
            session_lifetime(),
        );
        self.session_repository.add(&session).await?;

        Ok(Response {
            session_token,
            admin_id: admin.id(),
            expires_at: session.expires_at(),
            admin: AdminIdentity::of(&admin),
        })
    }
}
