use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::authentication::{
            refresh_session::{command::Command, response::Response},
            session_policy::SESSION_IDLE_TTL_MINUTES,
        },
    },
    domain::{
        entities::{
            audit_action::USER_SESSION_REFRESHED,
            audit_event::{ActorType, AuditEvent, Outcome},
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    // Slide a live pre-authorization session's idle window forward because the user
    // said they are still there.
    //
    // This is deliberately a user-gesture endpoint, never something the SPA polls: an
    // idle timeout that renews itself in the background is not an idle timeout. Two
    // things keep it from becoming an immortality grant:
    //
    //   * the absolute ceiling (`AuthSession::extend` clamps to it and refuses once it
    //     has passed), which bounds total lifetime no matter how often this is called;
    //   * token rotation, so the previous sessionCode dies here.
    //
    // It does NOT re-check the password, and it must not: the session already carries a
    // proven authentication. It also does not clear a pending MFA gate — a session that
    // is mid-MFA refreshes as itself, still pending.
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        // 1. Resolve the client by its public id → tenant. Inactive clients vanish.
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        let tenant_id = client.tenant_id();

        // 2. Same generic failure for every rejection reason, matching authorize_session:
        //    a caller must not be able to tell "expired" from "replayed" from "wrong
        //    client" — that distinction is a probing oracle.
        let invalid = || DomainError::AuthenticationError("invalid or expired session".to_string());

        let mut session = uow
            .auth_session_repository()
            .find_by_token_hash(tenant_id, &sha256_hex(&command.session_code))
            .await?
            .ok_or_else(invalid)?;

        // A session that has already been spent for an authorization code was burned by
        // `expire_now`, so it fails `is_expired` here too — a spent code cannot be
        // resurrected by refreshing it.
        if session.is_expired(now) || !session.is_authenticated() {
            return Err(invalid());
        }
        if session.pending_client_id() != client.id() {
            return Err(invalid());
        }

        let user_id = session.user_id().ok_or_else(invalid)?;

        // 3. A session whose user was deactivated mid-flow must not be extendable.
        let user = uow
            .user_repository()
            .find_by_id(user_id)
            .await?
            .filter(|u| u.tenant_id() == tenant_id && u.is_active())
            .ok_or_else(invalid)?;

        // 4. Slide the idle window, clamped to the ceiling. `false` means the ceiling has
        //    passed: there is no time left to hand out, so this is a rejection, not a
        //    no-op that would hand back a deadline in the past.
        if !session.extend(now, Duration::minutes(SESSION_IDLE_TTL_MINUTES)) {
            return Err(invalid());
        }

        // 5. Rotate: the raw token is returned exactly once, only the hash is stored.
        let raw_session_token = generate_opaque_token();
        session.rotate_token(sha256_hex(&raw_session_token));

        uow.auth_session_repository().update(&session).await?;

        let event = AuditEvent::new(
            Some(tenant_id),
            ActorType::User,
            Some(user.id()),
            USER_SESSION_REFRESHED,
            Outcome::Success,
        )
        .with_metadata(serde_json::json!({
            "client_id": client.id(),
            "expires_at": session.expires_at(),
            "absolute_expires_at": session.absolute_expires_at(),
        }));
        uow.audit_event_repository().record(&event).await?;

        uow.commit().await?;

        Ok(Response {
            session_code: raw_session_token,
            session_expires_at: session.expires_at(),
            session_absolute_expires_at: session.absolute_expires_at(),
        })
    }
}
