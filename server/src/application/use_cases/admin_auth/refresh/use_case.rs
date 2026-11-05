use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::shared::token_policy::REFRESH_TOKEN_TTL_DAYS;
use crate::{
    application::{
        ports::{admin_token_issuer::AdminTokenIssuer, audit_logger::AuditLogger},
        use_cases::admin_auth::refresh::{command::Command, response::Response},
    },
    domain::{
        entities::{
            audit_action::{ADMIN_TOKEN_REFRESHED, ADMIN_TOKEN_REUSE_DETECTED},
            audit_event::{ActorType, AuditEvent, Outcome},
        },
        errors::DomainError,
        repositories::{
            admin_refresh_token_repository::AdminRefreshTokenRepository,
            admin_user_repository::AdminUserRepository,
        },
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// Exchange a valid admin refresh token for a fresh access token, ROTATING the
// refresh token (mint a successor, invalidate the presented one). Core security
// property: REUSE DETECTION — presenting an already-rotated/revoked token signals
// theft and revokes the whole family. Every failure returns the same generic
// `AuthenticationError` (→ 401) so the dashboard simply bounces to /login.
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
    refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
    token_issuer: Arc<dyn AdminTokenIssuer>,
    audit_logger: Arc<dyn AuditLogger>,
}

impl UseCase {
    pub fn new(
        admin_repository: Arc<dyn AdminUserRepository>,
        refresh_token_repository: Arc<dyn AdminRefreshTokenRepository>,
        token_issuer: Arc<dyn AdminTokenIssuer>,
        audit_logger: Arc<dyn AuditLogger>,
    ) -> Self {
        Self {
            admin_repository,
            refresh_token_repository,
            token_issuer,
            audit_logger,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let now = Utc::now();
        let invalid = || DomainError::AuthenticationError("invalid_token".to_string());

        // 1. Look the presented refresh token up by hash.
        let presented = self
            .refresh_token_repository
            .find_by_token_hash(&sha256_hex(&command.refresh_token))
            .await?
            .ok_or_else(invalid)?;

        // 2. Reuse detection: an already rotated/revoked token presented again is the
        //    theft signal — revoke the entire family and refuse.
        if !presented.is_active(now) {
            if presented.rotated_at().is_some() || presented.revoked_at().is_some() {
                self.refresh_token_repository
                    .revoke_family(presented.family_id())
                    .await?;

                let event = AuditEvent::new(
                    None,
                    ActorType::Admin,
                    Some(presented.admin_id()),
                    ADMIN_TOKEN_REUSE_DETECTED,
                    Outcome::Failure,
                );
                let _ = self.audit_logger.record(event).await;
            }
            return Err(invalid());
        }

        // 3. Re-validate the admin account: it must still exist and be active. If not,
        //    kill the family (a deactivated admin's sessions end everywhere).
        let admin = match self
            .admin_repository
            .find_by_id(presented.admin_id())
            .await?
        {
            Some(admin) if admin.is_active() => admin,
            _ => {
                self.refresh_token_repository
                    .revoke_family(presented.family_id())
                    .await?;
                return Err(invalid());
            }
        };

        // 4. Rotate: mark the presented token rotated and mint its successor.
        let raw_new = generate_opaque_token();
        let mut presented = presented;
        let new_token = presented.rotate(
            sha256_hex(&raw_new),
            now + Duration::days(REFRESH_TOKEN_TTL_DAYS),
            now,
        );
        self.refresh_token_repository
            .rotate(&presented, &new_token)
            .await?;

        // 5. Re-mint the access token for the (still-valid) admin.
        let issued = self.token_issuer.issue(
            admin.id(),
            admin.email(),
            admin.name(),
            admin.organization_id(),
        )?;

        let event = AuditEvent::new(
            None,
            ActorType::Admin,
            Some(admin.id()),
            ADMIN_TOKEN_REFRESHED,
            Outcome::Success,
        );
        let _ = self.audit_logger.record(event).await;

        Ok(Response {
            access_token: issued.token,
            token_type: "Bearer".to_string(),
            expires_in: issued.expires_in,
            refresh_token: raw_new,
        })
    }
}
