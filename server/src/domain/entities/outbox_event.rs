use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::domain_event_type::DomainEventType;

// One fact that happened, recorded in the SAME database transaction as the change
// it describes. That co-commit is the entire point of the type: an OutboxEvent
// exists if and only if the user creation/update/deletion it announces committed.
//
// The dispatcher later fans each unpublished event out to the tenant's matching
// subscriptions (one WebhookDelivery per target) and stamps `published_at`.
//
// `payload` is the already-rendered public shape sent to consumers, frozen at
// enqueue time. Rendering it here rather than at delivery time means a later
// change to the user row cannot rewrite history a consumer is mid-way through
// receiving, and a retry days later replays exactly what the first attempt sent.
#[derive(Debug, Clone)]
pub struct OutboxEvent {
    id: Uuid,
    tenant_id: Uuid,
    event_type: DomainEventType,
    payload: serde_json::Value,
    occurred_at: DateTime<Utc>,
    published_at: Option<DateTime<Utc>>,
}

impl OutboxEvent {
    pub fn new(tenant_id: Uuid, event_type: DomainEventType, payload: serde_json::Value) -> Self {
        OutboxEvent {
            // uuid_v7 is time-sortable, so claiming "oldest unpublished first" is an
            // ORDER BY id — no extra index on occurred_at needed. It is also the
            // idempotency key consumers dedupe on.
            id: Uuid::now_v7(),
            tenant_id,
            event_type,
            payload,
            occurred_at: Utc::now(),
            published_at: None,
        }
    }

    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        event_type: DomainEventType,
        payload: serde_json::Value,
        occurred_at: DateTime<Utc>,
        published_at: Option<DateTime<Utc>>,
    ) -> Self {
        OutboxEvent {
            id,
            tenant_id,
            event_type,
            payload,
            occurred_at,
            published_at,
        }
    }

    pub fn mark_published(&mut self, at: DateTime<Utc>) {
        self.published_at = Some(at);
    }

    pub fn is_published(&self) -> bool {
        self.published_at.is_some()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn event_type(&self) -> DomainEventType {
        self.event_type
    }
    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
    pub fn published_at(&self) -> Option<DateTime<Utc>> {
        self.published_at
    }
}
