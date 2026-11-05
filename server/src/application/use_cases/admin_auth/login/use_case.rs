use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::ports::password_hasher::PasswordHasher;
use crate::application::shared::token_policy::REFRESH_TOKEN_TTL_DAYS;
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
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// How long an admin refresh token stays valid. The access token is short-lived;
// the refresh token lets the dashboard renew it silently for up to this window
// before the operator must sign in again.

// Verify an admin's email + password and, on success, mint an admin access token
// plus an opaque refresh token (stored hashed, starting its own rotation family).
// Failures return a single generic error (`AuthenticationError`) so the response
// cannot distinguish "unknown email" from "wrong password" from "deactivated
// account" — no user enumeration.
//
// BRUTE-FORCE PROTECTION: this endpoint is covered by the strict per-IP rate-limit
// bucket in `routes.rs` (10 burst, then one attempt every 2s), NOT by the adaptive
// risk engine that guards tenant logins.
//
// The risk engine is tenant-scoped by construction — `RiskEvaluationInput.tenant_id`
// is non-null and both `login_events` and `security_blocks` carry a NOT NULL foreign
// key to `tenants` — while a platform admin belongs to an organization and has no
// tenant. Extending it here means either a reserved sentinel tenant row or making
// `tenant_id` nullable across those tables, their repositories and every velocity
// query. That is a real change, not a wiring change, so it is tracked separately
// rather than bolted on here.
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
    token_issuer: Arc<dyn AdminTokenIssuer>,
    // Server-wide Argon2 pepper, matching what `create_admin` hashed with.
    password_pepper: String,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl UseCase {
    pub fn new(
        admin_repository: Arc<dyn AdminUserRepository>,
        refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
        token_issuer: Arc<dyn AdminTokenIssuer>,
        password_pepper: String,
        password_hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            admin_repository,
            refresh_token_repository,
            token_issuer,
            password_pepper,
            password_hasher,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let invalid = || DomainError::AuthenticationError("invalid credentials".to_string());

        // An unknown or inactive admin still pays one Argon2 verification (against
        // a dummy hash), so the response time does not reveal which admin emails
        // exist. Every failure is the same generic error.
        let admin = match self.admin_repository.find_by_email(&command.email).await? {
            Some(admin) if admin.is_active() => admin,
            _ => {
                self.password_hasher.verify_dummy(&command.password).await;
                return Err(invalid());
            }
        };

        if !self
            .password_hasher
            .verify(
                admin.password_hash(),
                &command.password,
                &self.password_pepper,
            )
            .await
        {
            return Err(invalid());
        }

        let issued = self.token_issuer.issue(
            admin.id(),
            admin.email(),
            admin.name(),
            admin.organization_id(),
        )?;

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
