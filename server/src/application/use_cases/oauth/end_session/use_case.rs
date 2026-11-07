use std::sync::Arc;

use chrono::Utc;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    application::{
        ports::{token_verifier::TokenVerifier, unit_of_work::UnitOfWorkFactory},
        shared::backchannel_logout::{EndedSessions, announce_ended_sessions},
        use_cases::oauth::end_session::{command::Command, response::Response},
    },
    domain::{
        entities::{
            audit_action::USER_LOGOUT,
            audit_event::{ActorType, AuditEvent, Outcome},
        },
        errors::DomainError,
    },
    utils::oauth_crypto::unverified_jwt_claims,
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    verifier: Arc<dyn TokenVerifier>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        verifier: Arc<dyn TokenVerifier>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            verifier,
        }
    }

    // Signs the user out of EVERY app in the tenant, not only the one asking.
    //
    // This OP keeps no session cookie of its own: each app's sign-in is a separate
    // refresh-token family. "The user's session at the OP" is therefore the set of all
    // their families, and logging out of it ends them all; the other apps learn about
    // it through back-channel logout. The asking app ended its own session already, so
    // it is not notified.
    //
    // Only a LIVE session may do this: the hint's `sid` must name a family of this
    // user, with this client, that has not ended. That stops an old or leaked ID token
    // from being replayed to sign the user out everywhere, long after the session it
    // came from is over. A hint for a session that already ended is not an error — the
    // user is signed out of it either way — it just ends nothing more.
    //
    // Errors never redirect: until the hint and the redirect URI are both validated,
    // there is no address we may trust with the browser.
    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let hint = command
            .id_token_hint
            .filter(|h| !h.is_empty())
            .ok_or_else(|| DomainError::InvalidRequest("id_token_hint is required".to_string()))?;

        // 1. Which client issued the request: the hint's audience (unverified, only to
        //    pick the client and its tenant's keys; verified below).
        let audience = unverified_jwt_claims(&hint)
            .as_ref()
            .and_then(audience_of)
            .ok_or_else(|| DomainError::InvalidRequest("malformed id_token_hint".to_string()))?;
        if let Some(client_id) = command.client_id.as_deref().filter(|c| !c.is_empty())
            && client_id != audience
        {
            return Err(DomainError::InvalidRequest(
                "client_id does not match the id_token_hint audience".to_string(),
            ));
        }

        let uow = self.unit_of_work_factory.begin().await?;
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&audience)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;
        let tenant_id = client.tenant_id();

        // 2. Verify the hint for real: signed by this tenant, by this issuer, for this
        //    client. Its expiry is deliberately not checked (see TokenVerifier).
        let keys = uow
            .signing_key_repository()
            .find_published(tenant_id)
            .await?;
        let verified = self
            .verifier
            .verify_id_token_hint(&hint, &keys)
            .await
            .map_err(|_| DomainError::InvalidRequest("invalid id_token_hint".to_string()))?;
        if audience_of(&verified.claims).as_deref() != Some(client.client_id().to_string().as_str())
        {
            return Err(DomainError::InvalidRequest(
                "id_token_hint was not issued to this client".to_string(),
            ));
        }
        let user_id = Uuid::parse_str(&verified.subject)
            .map_err(|_| DomainError::InvalidRequest("invalid id_token_hint".to_string()))?;
        let sid = verified
            .claims
            .get("sid")
            .and_then(Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok());

        // 3. The redirect, exact match only (same rule as redirect_uris).
        let post_logout_redirect_uri =
            match command.post_logout_redirect_uri.filter(|u| !u.is_empty()) {
                Some(uri) if client.is_post_logout_redirect_allowed(&uri) => Some(uri),
                Some(_) => {
                    return Err(DomainError::InvalidRequest(
                        "post_logout_redirect_uri is not registered for this client".to_string(),
                    ));
                }
                None => None,
            };

        // 4. End every session, if the hint's own session is still live.
        let live = uow
            .refresh_token_repository()
            .find_active_families_by_user(tenant_id, user_id, Utc::now())
            .await?;
        let from_live_session = sid.filter(|sid| {
            live.iter()
                .any(|f| f.family_id == *sid && f.client_id == client.id())
        });

        let mut sessions_ended = 0;
        if let Some(sid) = from_live_session {
            announce_ended_sessions(&*uow, tenant_id, user_id, EndedSessions::AllExcept(sid))
                .await?;
            uow.refresh_token_repository()
                .revoke_by_user(tenant_id, user_id)
                .await?;
            sessions_ended = live.len();

            let event = AuditEvent::new(
                Some(tenant_id),
                ActorType::User,
                Some(user_id),
                USER_LOGOUT,
                Outcome::Success,
            )
            .with_request_context(command.ip_address, command.user_agent)
            .with_metadata(json!({
                "client_id": client.client_id(),
                "sessions_ended": sessions_ended,
            }));
            uow.audit_event_repository().record(&event).await?;
            uow.commit().await?;
        }

        Ok(Response {
            post_logout_redirect_uri,
            state: command.state.filter(|s| !s.is_empty()),
            sessions_ended,
        })
    }
}

// `aud` is a string, or an array holding exactly one (an ID token from this OP has
// a single audience). Anything else names no client.
fn audience_of(claims: &Value) -> Option<String> {
    match claims.get("aud")? {
        Value::String(aud) => Some(aud.clone()),
        Value::Array(auds) if auds.len() == 1 => auds[0].as_str().map(str::to_string),
        _ => None,
    }
}
