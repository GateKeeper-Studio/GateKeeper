use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::{
    application::{
        ports::admin_token_issuer::AdminTokenIssuer,
        services::admin_authenticator::{AdminAuthenticator, AdminSignIn},
        shared::token_policy::REFRESH_TOKEN_TTL_DAYS,
        use_cases::admin_auth::login::{
            command::Command,
            response::{Outcome, Response},
        },
    },
    domain::{
        entities::admin_refresh_token::AdminRefreshToken, errors::DomainError,
        repositories::admin_refresh_token_repository::AdminRefreshTokenRepository,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// Bearer-token sign-in for API clients (`POST /admin/login`). Verifies the admin
// through `AdminAuthenticator` (generic failures, per-account lock, second factor)
// and, on success, mints an admin access token plus an opaque refresh token (stored
// hashed, starting its own rotation family). The console does not use this: it
// signs in with a cookie session (`admin_web_session::start`).
//
// BRUTE-FORCE PROTECTION: the strict per-IP rate-limit bucket in `routes.rs`, plus
// the per-account lock in the authenticator. The adaptive risk engine is not
// involved: it is tenant-scoped by construction (`login_events` and
// `security_blocks` carry a NOT NULL tenant), and a platform admin has no tenant.
pub struct UseCase {
    authenticator: Arc<AdminAuthenticator>,
    refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
    token_issuer: Arc<dyn AdminTokenIssuer>,
}

impl UseCase {
    pub fn new(
        authenticator: Arc<AdminAuthenticator>,
        refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
        token_issuer: Arc<dyn AdminTokenIssuer>,
    ) -> Self {
        Self {
            authenticator,
            refresh_token_repository,
            token_issuer,
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

        let setup_required = self.authenticator.mfa_setup_required(&admin).await?;
        let issued = self.token_issuer.issue(
            admin.id(),
            admin.email(),
            admin.name(),
            admin.organization_id(),
            setup_required,
        )?;

        // Mint and persist an opaque refresh token (hashed) opening a new family.
        let raw_refresh = generate_opaque_token();
        let refresh_token = AdminRefreshToken::new(
            admin.id(),
            sha256_hex(&raw_refresh),
            Utc::now() + Duration::days(REFRESH_TOKEN_TTL_DAYS),
        );
        self.refresh_token_repository.add(&refresh_token).await?;

        Ok(Outcome::SignedIn {
            admin_id: admin.id(),
            response: Response {
                access_token: issued.token,
                token_type: "Bearer".to_string(),
                expires_in: issued.expires_in,
                refresh_token: raw_refresh,
            },
        })
    }
}
