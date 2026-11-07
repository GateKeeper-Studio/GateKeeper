use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::webhook_delivery::WebhookDelivery, errors::DomainError};

#[async_trait]
pub trait WebhookDeliveryRepository: Send + Sync {
    // Dispatcher phase 1. Inserts one row per (event, subscription), ignoring rows
    // that already exist.
    //
    // The "ignore conflicts" part is what makes fan-out idempotent: if the worker
    // dies after inserting deliveries but before stamping the event as published,
    // the next tick re-runs the phase and inserts nothing new instead of
    // double-delivering.
    async fn insert_many_ignoring_conflicts(
        &self,
        deliveries: &[WebhookDelivery],
    ) -> Result<(), DomainError>;

    // Dispatcher phase 2. Takes pending deliveries whose backoff has elapsed.
    // Like claim_unpublished, the SQL implementation MUST use
    // `FOR UPDATE SKIP LOCKED` so replicas don't collide.
    async fn claim_due(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>, DomainError>;

    async fn update(&self, delivery: &WebhookDelivery) -> Result<(), DomainError>;

    // Admin delivery log — the debugging surface for "why hasn't my endpoint
    // received anything". Webhook deliveries only: back-channel logout deliveries
    // share the table but not this log (they have no subscription to show).
    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<WebhookDelivery>, DomainError>;

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError>;
}
