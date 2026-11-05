use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::{
        ports::{
            audit_logger::AuditLogger,
            risk_engine::{RiskEngine, RiskEvaluationInput},
            token_signer::TokenSigner,
            unit_of_work::UnitOfWorkFactory,
        },
        use_cases::oauth::{
            token_refresh::{command::Command, response::Response},
            user_claims::resolve_user_claims,
        },
    },
    domain::{
        entities::{
            audit_action::{TOKEN_REFRESHED, TOKEN_REUSE_DETECTED},
            audit_event::{ActorType, AuditEvent, Outcome},
            login_event::RiskDecision,
        },
        errors::DomainError,
    },
    utils::{
        oauth_crypto::{generate_opaque_token, sha256_hex},
        password_hasher::compare_password,
    },
};

const ACCESS_TOKEN_TTL_SECONDS: i64 = 900; // 15 minutes
const REFRESH_TOKEN_TTL_DAYS: i64 = 30;
/// Default grace window (seconds) after a token is rotated during which presenting it again
/// is treated as a legitimate concurrent refresh / retry (reissue a sibling) rather than
/// token reuse (revoke the whole family). Bounds how long a just-rotated token stays usable,
/// so keep it small — the standard rotation-leeway tradeoff (cf. Auth0's reuse interval).
/// Overridable per environment via `REFRESH_REUSE_LEEWAY_SECONDS` (see `routes.rs`).
pub const DEFAULT_REFRESH_REUSE_LEEWAY_SECONDS: i64 = 10;

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    signer: Arc<dyn TokenSigner>,
    issuer: String,
    audit_logger: Arc<dyn AuditLogger>,
    // See DEFAULT_REFRESH_REUSE_LEEWAY_SECONDS; injected so it's tunable per environment.
    reuse_leeway_seconds: i64,
    // Same engine the interactive login uses, so a refresh from a new country / an
    // impossible-travel hop / a blocked IP is geo-enriched, recorded as a login event,
    // and (on a critical decision) refused.
    risk_engine: Arc<dyn RiskEngine>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        signer: Arc<dyn TokenSigner>,
        issuer: String,
        audit_logger: Arc<dyn AuditLogger>,
        reuse_leeway_seconds: i64,
        risk_engine: Arc<dyn RiskEngine>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            signer,
            issuer,
            audit_logger,
            reuse_leeway_seconds,
            risk_engine,
        }
    }

    // grant_type=refresh_token. Core security property: ROTATION with REUSE
    // DETECTION. Presenting an already-rotated token nukes the whole family.
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let now = Utc::now();
        let uow = self.unit_of_work_factory.begin().await?;

        // 1. Authenticate the client (confidential ⇒ verify secret).
        let client = uow
            .oauth_client_repository()
            .find_by_client_id(command.tenant_id, &command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        if client.is_confidential() {
            let tenant = uow
                .tenant_repository()
                .find_by_id(command.tenant_id)
                .await?
                .ok_or_else(|| DomainError::InvalidClient("unknown tenant".to_string()))?;

            let presented = command.client_secret.as_deref().unwrap_or("");
            let valid = match client.client_secret_hash() {
                Some(hash) => compare_password(hash, presented, tenant.password_hash_secret()),
                None => false,
            };
            if !valid {
                return Err(DomainError::InvalidClient(
                    "invalid client secret".to_string(),
                ));
            }
        }

        // 2. Look the presented refresh token up by hash.
        let presented = uow
            .refresh_token_repository()
            .find_by_token_hash(command.tenant_id, &sha256_hex(&command.refresh_token))
            .await?
            .ok_or_else(|| DomainError::InvalidGrant("unknown refresh token".to_string()))?;

        // The token must belong to the authenticated client.
        if presented.client_id() != client.id() {
            return Err(DomainError::InvalidGrant("client mismatch".to_string()));
        }

        // 3. Reuse handling, with a short rotation leeway. An inactive token is normally
        //    theft (already rotated) or dead (revoked/expired). BUT a token that was *just*
        //    rotated and presented again within REFRESH_REUSE_LEEWAY_SECONDS is almost
        //    always a legitimate concurrent refresh or a network retry racing the rotation
        //    — not theft. In that window we reissue a sibling in the same family instead of
        //    nuking it. Outside the window (or if revoked), keep the kill-the-family
        //    behavior. A revoked token never gets grace.
        let active = presented.is_active(now);
        if !active {
            let within_leeway = match presented.rotated_at() {
                Some(rotated_at) => {
                    presented.revoked_at().is_none()
                        && now.signed_duration_since(rotated_at)
                            <= Duration::seconds(self.reuse_leeway_seconds)
                }
                None => false,
            };

            if !within_leeway {
                if presented.rotated_at().is_some() || presented.revoked_at().is_some() {
                    uow.refresh_token_repository()
                        .revoke_family(command.tenant_id, presented.family_id())
                        .await?;
                    uow.commit().await?;

                    // High-signal security event: an already-rotated/revoked token was
                    // presented outside the leeway — likely theft. The family was revoked.
                    let event = AuditEvent::new(
                        Some(command.tenant_id),
                        ActorType::User,
                        Some(presented.user_id()),
                        TOKEN_REUSE_DETECTED,
                        Outcome::Failure,
                    );
                    let _ = self.audit_logger.record(event).await;
                }
                return Err(DomainError::InvalidGrant(
                    "refresh token reuse detected or expired".to_string(),
                ));
            }
        }

        // 3b. Risk evaluation for the refresh (same engine as the interactive login):
        //     geo-enrich, record a login_event so the refresh appears in the security
        //     views, and refuse on a critical decision. This is a non-interactive path,
        //     so we never attempt MFA step-up — only a temporary block forces the client
        //     back to a full re-authentication. The engine persists on its own connection.
        let risk = self
            .risk_engine
            .evaluate(RiskEvaluationInput {
                tenant_id: command.tenant_id,
                user_id: Some(presented.user_id()),
                client_id: Some(client.id()),
                username_normalized: None,
                ip_address: command.ip_address.clone(),
                user_agent: command.user_agent.clone(),
                credentials_valid: true,
                pre_auth_blocked: false,
                failure_reason: None,
                timestamp: now,
            })
            .await?;
        if risk.decision == RiskDecision::BlockTemporarily {
            return Err(DomainError::InvalidGrant(
                "refresh blocked by risk policy".to_string(),
            ));
        }

        // 4. Mint the successor. An active token rotates (the presented token is stamped
        //    rotated_at and replaced). A within-leeway token is left untouched and instead
        //    gets a fresh sibling in the same family, so its leeway window doesn't slide.
        let raw_new = generate_opaque_token();
        let new_hash = sha256_hex(&raw_new);
        let new_expires_at = now + Duration::days(REFRESH_TOKEN_TTL_DAYS);

        let new_token = if active {
            let mut presented = presented;
            let child = presented.rotate(new_hash, new_expires_at, now);
            uow.refresh_token_repository()
                .rotate(&presented, &child)
                .await?;
            child
        } else {
            let child = presented.reissue_within_family(new_hash, new_expires_at, now);
            uow.refresh_token_repository().add(&child).await?;
            child
        };

        // 5. Re-mint the access (+ id) token with the active signing key.
        let key = uow
            .signing_key_repository()
            .find_active(command.tenant_id)
            .await?
            .ok_or_else(|| DomainError::InfrastructureError("no active signing key".to_string()))?;

        // Re-resolve role/group names so a refreshed token reflects the user's
        // CURRENT authorization context, not whatever it was at original sign-in.
        let (roles, groups) =
            resolve_user_claims(&*uow, command.tenant_id, new_token.user_id()).await?;

        let scopes: Vec<String> = new_token.scopes().to_vec();
        let scope_str = scopes.join(" ");
        let iat = now.timestamp();
        let exp = iat + ACCESS_TOKEN_TTL_SECONDS;

        let access_claims = json!({
          "iss": self.issuer,
          "sub": new_token.user_id().to_string(),
          "aud": client.client_id(),
          "tid": command.tenant_id.to_string(),
          "scope": scope_str,
          "roles": roles,
          "groups": groups,
          "iat": iat,
          "exp": exp,
          "jti": Uuid::now_v7().to_string(),
        });
        let access_token = self.signer.sign(&key, &access_claims).await?;

        let id_token = if scopes.iter().any(|s| s == "openid") {
            let id_claims = json!({
              "iss": self.issuer,
              "sub": new_token.user_id().to_string(),
              "aud": client.client_id(),
              "roles": roles,
              "groups": groups,
              "iat": iat,
              "exp": exp,
            });
            Some(self.signer.sign(&key, &id_claims).await?)
        } else {
            None
        };

        uow.commit().await?;

        // Audit: a refresh token was rotated (session continued).
        let event = AuditEvent::new(
            Some(command.tenant_id),
            ActorType::User,
            Some(new_token.user_id()),
            TOKEN_REFRESHED,
            Outcome::Success,
        )
        .with_metadata(serde_json::json!({ "client_id": client.client_id(), "grace": !active }));
        let _ = self.audit_logger.record(event).await;

        Ok(Response {
            access_token,
            id_token,
            refresh_token: raw_new,
            token_type: "Bearer".to_string(),
            expires_in: ACCESS_TOKEN_TTL_SECONDS,
            scope: scope_str,
        })
    }
}
