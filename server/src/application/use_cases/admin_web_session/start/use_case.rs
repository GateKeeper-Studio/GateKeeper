use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        services::admin_authenticator::{AdminAuthenticator, AdminSignIn},
        use_cases::admin_web_session::{
            identity::AdminIdentity,
            session_lifetime,
            start::{
                command::Command,
                response::{Outcome, Response},
            },
        },
    },
    domain::{
        entities::admin_web_session::AdminWebSession, errors::DomainError,
        repositories::admin_web_session_repository::AdminWebSessionRepository,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// Signs a platform admin in to the console: verifies them exactly like
// `POST /admin/login` (same `AdminAuthenticator`), then opens a browser session.
// Unlike `/admin/login` it hands out no bearer token and no refresh token: the
// caller gets an opaque session id for an httpOnly cookie, and only its hash is
// stored. A new id is minted on every sign-in, so a session id planted before
// sign-in is never upgraded (no session fixation).
pub struct UseCase {
    authenticator: Arc<AdminAuthenticator>,
    session_repository: Arc<dyn AdminWebSessionRepository>,
}

impl UseCase {
    pub fn new(
        authenticator: Arc<AdminAuthenticator>,
        session_repository: Arc<dyn AdminWebSessionRepository>,
    ) -> Self {
        Self {
            authenticator,
            session_repository,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Outcome, DomainError> {
        let admin = match self
            .authenticator
            .sign_in(
                &command.email,
                &command.password,
                command.mfa_code.as_deref(),
            )
            .await?
        {
            AdminSignIn::Authenticated(admin) => admin,
            AdminSignIn::SecondFactorRequired(admin) => {
                return Ok(Outcome::SecondFactorRequired {
                    admin_id: admin.id(),
                });
            }
        };

        // 256 random bits; the browser keeps it, the database keeps its SHA-256.
        let session_token = generate_opaque_token();
        let session = AdminWebSession::new(
            admin.id(),
            sha256_hex(&session_token),
            Utc::now(),
            session_lifetime(),
        );
        self.session_repository.add(&session).await?;

        Ok(Outcome::Started(Response {
            session_token,
            admin_id: admin.id(),
            expires_at: session.expires_at(),
            admin: AdminIdentity::of(&admin)
                .with_mfa_setup_required(self.authenticator.mfa_setup_required(&admin).await?),
        }))
    }
}
