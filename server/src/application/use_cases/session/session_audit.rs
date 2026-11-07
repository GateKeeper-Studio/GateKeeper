use uuid::Uuid;

use crate::{
    application::shared::actor::Actor,
    domain::entities::{
        audit_action::USER_SESSION_REVOKED,
        audit_event::{ActorType, AuditEvent, Outcome},
    },
};

// The audit row for ending a session. A user ending their own session is a `User`
// action; anyone else (a tenant or platform admin, or the system) is an `Admin` one.
pub(super) fn session_revoked(
    actor: &Actor,
    tenant_id: Uuid,
    user_id: Uuid,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> AuditEvent {
    let actor_type = match actor {
        Actor::System => ActorType::System,
        _ if actor.id() == Some(user_id) => ActorType::User,
        _ => ActorType::Admin,
    };
    AuditEvent::new(
        Some(tenant_id),
        actor_type,
        actor.id(),
        USER_SESSION_REVOKED,
        Outcome::Success,
    )
    .with_request_context(ip_address, user_agent)
}
