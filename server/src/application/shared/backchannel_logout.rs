// OIDC Back-Channel Logout 1.0: telling the RPs that a user's session with them is
// over, so they can end their own session too.
//
// Two halves, both here so the payload shape has one owner:
//
//   * `announce_ended_sessions` — called by every use case that ends sessions, in
//     ITS transaction and BEFORE it revokes the refresh tokens (afterwards there is
//     nothing left to find). It enqueues one outbox event per ended session whose
//     client registered a `backchannel_logout_uri`. Same co-commit guarantee as the
//     user webhooks: the notification exists if and only if the sign-out committed.
//
//   * `logout_token_claims` — what the dispatcher signs (`typ: logout+jwt`) for each
//     delivery ATTEMPT. The token is minted per attempt, not at enqueue time, because
//     it must be short-lived (`exp`) while a retry may go out hours later.
//
// A session here is a refresh-token family, and `sid` is its family id: the same
// value the ID tokens of that session carry (see `token_service.rs`).
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    application::ports::unit_of_work::UnitOfWork,
    domain::{
        entities::{domain_event_type::DomainEventType, outbox_event::OutboxEvent},
        errors::DomainError,
        repositories::refresh_token_repository::RefreshTokenFamily,
    },
};

// The `events` member every logout token carries (Back-Channel Logout 1.0 §2.4).
pub const BACKCHANNEL_LOGOUT_EVENT: &str = "http://schemas.openid.net/event/backchannel-logout";

// Lifetime of one logout token. The spec recommends two minutes or less: the token
// is consumed by a server-to-server POST right after it is signed.
pub const LOGOUT_TOKEN_TTL_SECONDS: i64 = 120;

// Which of a user's live sessions are ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndedSessions {
    // Every session (sign out everywhere, password change or reset, erasure, ...).
    All,
    // Every session but this one: RP-initiated logout, where the RP that asked
    // has already ended its own session and needs no notification.
    AllExcept(Uuid),
    // One session (family id): a single sign-out, or reuse detection.
    Family(Uuid),
    // Every session with one client (oauth_clients.id): consent revocation.
    Client(Uuid),
}

impl EndedSessions {
    fn includes(&self, family: &RefreshTokenFamily) -> bool {
        match *self {
            EndedSessions::All => true,
            EndedSessions::AllExcept(sid) => family.family_id != sid,
            EndedSessions::Family(sid) => family.family_id == sid,
            EndedSessions::Client(client_id) => family.client_id == client_id,
        }
    }
}

// One logout notification, as stored in the outbox payload. `client_id` is the
// internal oauth_clients.id; the public client_id goes into the token's `aud`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogoutNotice {
    pub client_id: Uuid,
    pub user_id: Uuid,
    pub sid: Uuid,
}

impl LogoutNotice {
    pub fn into_event(self, tenant_id: Uuid) -> OutboxEvent {
        OutboxEvent::new(
            tenant_id,
            DomainEventType::BackchannelLogout,
            json!({
                "client_id": self.client_id.to_string(),
                "sub": self.user_id.to_string(),
                "sid": self.sid.to_string(),
            }),
        )
    }

    // None for a payload this binary does not understand; the dispatcher then
    // publishes the event without delivering it rather than stalling the batch.
    pub fn from_payload(payload: &Value) -> Option<Self> {
        let uuid = |key: &str| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .and_then(|s| Uuid::parse_str(s).ok())
        };
        Some(LogoutNotice {
            client_id: uuid("client_id")?,
            user_id: uuid("sub")?,
            sid: uuid("sid")?,
        })
    }
}

// Enqueues a logout notification for each of the user's LIVE sessions that `which`
// selects and whose client wants one (active, with a back-channel logout URI).
// Returns the sessions that were live, notified or not, so a caller can report how
// many it ended.
//
// Must run before the caller revokes the tokens, inside the same unit of work.
// Sessions that had already ended are not live, so they are never announced twice.
pub async fn announce_ended_sessions(
    uow: &dyn UnitOfWork,
    tenant_id: Uuid,
    user_id: Uuid,
    which: EndedSessions,
) -> Result<Vec<RefreshTokenFamily>, DomainError> {
    let ended: Vec<RefreshTokenFamily> = uow
        .refresh_token_repository()
        .find_active_families_by_user(tenant_id, user_id, Utc::now())
        .await?
        .into_iter()
        .filter(|family| which.includes(family))
        .collect();

    // A user usually has several sessions with few clients: one lookup per client.
    let mut wants_notice: HashMap<Uuid, bool> = HashMap::new();
    for family in &ended {
        let wanted = match wants_notice.get(&family.client_id) {
            Some(wanted) => *wanted,
            None => {
                let wanted = uow
                    .oauth_client_repository()
                    .find_by_id(tenant_id, family.client_id)
                    .await?
                    .is_some_and(|c| c.is_active() && c.backchannel_logout_uri().is_some());
                wants_notice.insert(family.client_id, wanted);
                wanted
            }
        };
        if !wanted {
            continue;
        }

        let notice = LogoutNotice {
            client_id: family.client_id,
            user_id,
            sid: family.family_id,
        };
        uow.outbox_event_repository()
            .enqueue(&notice.into_event(tenant_id))
            .await?;
    }

    Ok(ended)
}

// The logout token's claims (Back-Channel Logout 1.0 §2.4). Both `sub` and `sid`
// are always present: an RP that tracks sessions ends exactly that one, an RP that
// only knows users ends the user's sessions. There is never a `nonce` (§2.4 forbids
// one) and `jti` is fresh per token, so an RP can reject a replay.
pub fn logout_token_claims(
    issuer: &str,
    audience: &str,
    notice: &LogoutNotice,
    now: DateTime<Utc>,
) -> Value {
    let iat = now.timestamp();
    json!({
        "iss": issuer,
        "aud": audience,
        "iat": iat,
        "exp": iat + LOGOUT_TOKEN_TTL_SECONDS,
        "jti": Uuid::now_v7().to_string(),
        "sub": notice.user_id.to_string(),
        "sid": notice.sid.to_string(),
        "events": { BACKCHANNEL_LOGOUT_EVENT: {} },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_payload_round_trips() {
        let notice = LogoutNotice {
            client_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            sid: Uuid::now_v7(),
        };
        let event = notice.into_event(Uuid::now_v7());

        assert_eq!(event.event_type(), DomainEventType::BackchannelLogout);
        assert_eq!(LogoutNotice::from_payload(event.payload()), Some(notice));
    }

    #[test]
    fn a_payload_missing_a_field_is_not_a_notice() {
        assert_eq!(
            LogoutNotice::from_payload(&json!({ "sub": Uuid::now_v7().to_string() })),
            None
        );
    }

    // Pins the claim set the spec requires, and that nothing else rides along.
    #[test]
    fn the_logout_token_has_exactly_the_spec_claims() {
        let notice = LogoutNotice {
            client_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            sid: Uuid::now_v7(),
        };
        let now = Utc::now();

        let claims = logout_token_claims("https://idp.example.com", "client-123", &notice, now);

        let mut keys: Vec<&str> = claims
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["aud", "events", "exp", "iat", "iss", "jti", "sid", "sub"]
        );
        assert_eq!(claims["iss"], "https://idp.example.com");
        assert_eq!(claims["aud"], "client-123");
        assert_eq!(claims["sub"], notice.user_id.to_string());
        assert_eq!(claims["sid"], notice.sid.to_string());
        assert_eq!(claims["events"], json!({ BACKCHANNEL_LOGOUT_EVENT: {} }));
        assert_eq!(
            claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
            LOGOUT_TOKEN_TTL_SECONDS
        );
        assert!(claims.get("nonce").is_none());
    }

    #[test]
    fn every_token_gets_its_own_jti() {
        let notice = LogoutNotice {
            client_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
            sid: Uuid::now_v7(),
        };
        let a = logout_token_claims("iss", "aud", &notice, Utc::now());
        let b = logout_token_claims("iss", "aud", &notice, Utc::now());
        assert_ne!(a["jti"], b["jti"]);
    }
}

// Every use case that ends sessions announces them. One test per distinct trigger,
// driven through the real use cases against the session fixture (two apps: one
// registered for back-channel logout, one not).
#[cfg(test)]
mod triggers {
    use chrono::Duration;
    use uuid::Uuid;

    use crate::{
        application::{
            shared::actor::Actor,
            use_cases::{
                compliance::erase_user, consent_grant::revoke_consent,
                tenant_user::delete_tenant_user_by_id,
            },
        },
        testing::session_fixture::build,
    };

    fn sids(notices: &[super::LogoutNotice]) -> Vec<Uuid> {
        let mut sids: Vec<Uuid> = notices.iter().map(|n| n.sid).collect();
        sids.sort_unstable();
        sids
    }

    // Revoking consent ends the sessions with THAT app only.
    #[tokio::test]
    async fn revoking_consent_announces_that_apps_sessions() {
        let fx = build();
        let with_app = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(2), 1);
        fx.seed_family(
            fx.tenant_id,
            fx.alice,
            fx.other_app.id(),
            Duration::hours(1),
            0,
        );
        fx.seed_family(fx.tenant_id, fx.bob, fx.app.id(), Duration::hours(1), 0);

        revoke_consent::use_case::UseCase::new(fx.factory.clone())
            .execute(
                &Actor::System,
                revoke_consent::command::Command {
                    tenant_id: fx.tenant_id,
                    user_id: fx.alice,
                    client_id: fx.app.id(),
                },
            )
            .await
            .unwrap();

        assert_eq!(sids(&fx.logout_notices()), vec![with_app.family_id]);
    }

    // Deleting a user cascades their refresh tokens away, so the announcement has to
    // happen first, in the same transaction.
    #[tokio::test]
    async fn deleting_a_user_announces_their_sessions() {
        let fx = build();
        let a = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(2), 0);
        let b = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);

        delete_tenant_user_by_id::use_case::UseCase::new(fx.factory.clone())
            .execute(
                &Actor::System,
                delete_tenant_user_by_id::command::Command {
                    tenant_id: fx.tenant_id,
                    user_id: fx.alice,
                },
            )
            .await
            .unwrap();

        let mut expected = vec![a.family_id, b.family_id];
        expected.sort_unstable();
        assert_eq!(sids(&fx.logout_notices()), expected);
        assert!(fx.logout_notices().iter().all(|n| n.user_id == fx.alice));
    }

    #[tokio::test]
    async fn erasing_a_user_announces_their_sessions() {
        let fx = build();
        let a = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(2), 0);
        fx.seed_family(fx.tenant_id, fx.bob, fx.app.id(), Duration::hours(1), 0);

        erase_user::use_case::UseCase::new(fx.factory.clone())
            .execute(
                &Actor::System,
                erase_user::command::Command {
                    tenant_id: fx.tenant_id,
                    user_id: fx.alice,
                },
            )
            .await
            .unwrap();

        assert_eq!(sids(&fx.logout_notices()), vec![a.family_id]);
    }

    // A refused sign-out (here: a user of another tenant) announces nothing.
    #[tokio::test]
    async fn a_refused_sign_out_leaves_no_notice() {
        let fx = build();
        fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);

        let err = delete_tenant_user_by_id::use_case::UseCase::new(fx.factory.clone())
            .execute(
                &Actor::System,
                delete_tenant_user_by_id::command::Command {
                    tenant_id: fx.other_tenant_id,
                    user_id: fx.alice,
                },
            )
            .await;

        assert!(err.is_err());
        assert!(fx.logout_notices().is_empty());
    }
}
