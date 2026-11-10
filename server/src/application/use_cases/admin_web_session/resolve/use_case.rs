use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::admin_token_issuer::AdminTokenIssuer,
        services::admin_authenticator::AdminAuthenticator,
        use_cases::admin_web_session::{
            identity::AdminIdentity,
            idle_timeout,
            resolve::{query::Query, response::Response},
            touch_granularity,
        },
    },
    domain::{
        errors::DomainError,
        repositories::{
            admin_user_repository::AdminUserRepository,
            admin_web_session_repository::AdminWebSessionRepository,
        },
    },
    utils::oauth_crypto::sha256_hex,
};

// Turns a console cookie into an authenticated admin for one request.
//
// The session stores no token, so there is nothing to refresh and nothing to steal
// from the database: each call mints a new short-lived admin access token, the same
// HS256 token `/admin/login` issues, which `require_admin` then verifies as usual.
// The admin account is re-read every time, so deactivating an admin, or suspending
// their organization, ends their console sessions on the next request (a bearer
// token stays valid until it expires).
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    session_repository: Arc<dyn AdminWebSessionRepository>,
    token_issuer: Arc<dyn AdminTokenIssuer>,
    authenticator: Arc<AdminAuthenticator>,
}

impl UseCase {
    pub fn new(
        admin_repository: Arc<dyn AdminUserRepository>,
        session_repository: Arc<dyn AdminWebSessionRepository>,
        token_issuer: Arc<dyn AdminTokenIssuer>,
        authenticator: Arc<AdminAuthenticator>,
    ) -> Self {
        Self {
            admin_repository,
            session_repository,
            token_issuer,
            authenticator,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let expired = || DomainError::AuthenticationError("session expired".to_string());
        let now = Utc::now();
        let token_hash = sha256_hex(&query.session_token);

        let session = self
            .session_repository
            .find_by_token_hash(&token_hash)
            .await?
            .ok_or_else(expired)?;

        if !session.is_live(now, idle_timeout()) {
            self.session_repository
                .delete_by_token_hash(&token_hash)
                .await?;
            return Err(expired());
        }

        let admin = match self.admin_repository.find_by_id(session.admin_id()).await? {
            Some(admin) if admin.can_sign_in() => admin,
            _ => {
                self.session_repository
                    .delete_by_token_hash(&token_hash)
                    .await?;
                return Err(expired());
            }
        };

        // Re-evaluated on every request, so turning the organization's requirement on
        // restricts existing sessions at once, and enrolling lifts it at once.
        let setup_required = self.authenticator.mfa_setup_required(&admin).await?;
        let issued = self.token_issuer.issue(
            admin.id(),
            admin.email(),
            admin.name(),
            admin.organization_id(),
            setup_required,
        )?;

        if session.needs_touch(now, touch_granularity()) {
            self.session_repository.touch(session.id(), now).await?;
        }

        Ok(Response {
            session_id: session.id(),
            admin: AdminIdentity::of(&admin).with_mfa_setup_required(setup_required),
            access_token: issued.token,
        })
    }
}
