use serde_json::json;
use uuid::Uuid;

use crate::domain::entities::{
    domain_event_type::{DomainEventType, UserEventSource},
    outbox_event::OutboxEvent,
    user::User,
};

// Builds the public event body for a user lifecycle change and wraps it in an
// OutboxEvent ready to enqueue.
//
// Every enqueue site funnels through here so all seven of them emit an identical
// `data` shape — a consumer writing one parser is the whole point of the feature,
// and hand-rolling the JSON per call site is how that quietly stops being true.
//
// PII NOTE: this body crosses a trust boundary — it carries the user's email and
// name to a third-party endpoint the tenant admin configured. That is the purpose
// of the feature, not an oversight, but it is why the target URL is validated as a
// security control (see use_cases/webhook_subscription/validation.rs) and why the
// secret is never reused across subscriptions.
pub fn user_event(
    event_type: DomainEventType,
    user: &User,
    source: Option<UserEventSource>,
) -> OutboxEvent {
    let profile = user.profile();

    let data = json!({
        "user_id": user.id().to_string(),
        "email": user.email(),
        "email_verified": user.is_email_verified(),
        "display_name": profile.and_then(|p| p.display_name().cloned()),
        "first_name": profile.map(|p| p.first_name().to_string()),
        "last_name": profile.map(|p| p.last_name().to_string()),
        "is_active": user.is_active(),
        "source": source.map(|s| s.as_str()),
        "created_at": user.created_at().to_rfc3339(),
        "updated_at": user.updated_at().map(|t| t.to_rfc3339()),
    });

    OutboxEvent::new(user.tenant_id(), event_type, wrap(event_type, data))
}

// A deletion has no User left to describe once the row is gone, so the id is all
// a consumer needs (and all we can honestly promise). Kept separate rather than
// making every field of `user_event` optional.
pub fn user_deleted_event(tenant_id: Uuid, user_id: Uuid) -> OutboxEvent {
    let data = json!({ "user_id": user_id.to_string() });
    OutboxEvent::new(
        tenant_id,
        DomainEventType::UserDeleted,
        wrap(DomainEventType::UserDeleted, data),
    )
}

// The envelope every consumer parses. `id` and `occurred_at` are filled in by the
// dispatcher at delivery time from the OutboxEvent itself — putting them here too
// would risk the two disagreeing.
fn wrap(event_type: DomainEventType, data: serde_json::Value) -> serde_json::Value {
    json!({
        "type": event_type.as_str(),
        "data": data,
    })
}
