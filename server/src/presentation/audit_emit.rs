use uuid::Uuid;

use crate::{
    domain::entities::audit_event::{ActorType, AuditEvent, Outcome},
    presentation::{
        middleware::auth::{AuthPrincipal, RequestContext},
        routes::AppState,
    },
};

// Shared helper for emitting a successful admin (CUD) audit event from a handler
// that already runs behind `require_admin` (so it has the principal + request
// context). Best-effort — a logging failure never affects the response.
pub async fn admin_action(
    state: &AppState,
    tenant_id: Uuid,
    principal: &AuthPrincipal,
    ctx: &RequestContext,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
) {
    let event = AuditEvent::new(
        Some(tenant_id),
        ActorType::Admin,
        principal.actor_id(),
        action,
        Outcome::Success,
    )
    .with_resource(resource_type, resource_id)
    .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
    let _ = state.audit_logger.record(event).await;
}
