use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::{
    application::{
        ports::admin_token_issuer::AdminTokenIssuer,
        use_cases::admin_auth::login::{command::Command, response::Response},
    },
    domain::{
        entities::admin_refresh_token::AdminRefreshToken,
        errors::DomainError,
        repositories::{
            admin_refresh_token_repository::AdminRefreshTokenRepository,
            admin_user_repository::AdminUserRepository,
        },
    },
    utils::{
        oauth_crypto::{generate_opaque_token, sha256_hex},
        password_hasher::compare_password,
    },
};

// How long an admin refresh token stays valid. The access token is short-lived;
// the refresh token lets the dashboard renew it silently for up to this window
// before the operator must sign in again.
const REFRESH_TOKEN_TTL_DAYS: i64 = 30;

// Verify an admin's email + password and, on success, mint an admin access token
// plus an opaque refresh token (stored hashed, starting its own rotation family).
// Failures return a single generic error (`AuthenticationError`) so the response
// cannot distinguish "unknown email" from "wrong password" from "deactivated
// account" — no user enumeration.
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
    token_issuer: Arc<dyn AdminTokenIssuer>,
    // Server-wide Argon2 pepper, matching what `create_admin` hashed with.
    password_pepper: String,
}

impl UseCase {
    pub fn new(
        admin_repository: Arc<dyn AdminUserRepository>,
        refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
        token_issuer: Arc<dyn AdminTokenIssuer>,
        password_pepper: String,
    ) -> Self {
        Self {
            admin_repository,
            refresh_token_repository,
            token_issuer,
            password_pepper,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let invalid = || DomainError::AuthenticationError("invalid credentials".to_string());

        let admin = self
            .admin_repository
            .find_by_email(&command.email)
            .await?
            .ok_or_else(invalid)?;

        if !admin.is_active() {
            return Err(invalid());
        }

        if !compare_password(
            admin.password_hash(),
            &command.password,
            &self.password_pepper,
        ) {
            return Err(invalid());
        }

        let issued = self
            .token_issuer
            .issue(admin.id(), admin.email(), admin.organization_id())?;

        // Mint and persist an opaque refresh token (hashed) opening a new family.
        let raw_refresh = generate_opaque_token();
        let refresh_token = AdminRefreshToken::new(
            admin.id(),
            sha256_hex(&raw_refresh),
            Utc::now() + Duration::days(REFRESH_TOKEN_TTL_DAYS),
        );
        self.refresh_token_repository.add(&refresh_token).await?;

        Ok(Response {
            access_token: issued.token,
            token_type: "Bearer".to_string(),
            expires_in: issued.expires_in,
            refresh_token: raw_refresh,
        })
    }
}
