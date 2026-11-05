use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::application::ports::secret_cipher::{SecretContext, SecretPurpose};
use crate::{
    application::ports::{
        secret_cipher::SecretCipher,
        unit_of_work::UnitOfWorkFactory,
        webhook_sender::{WebhookRequest, WebhookSender},
    },
    domain::{
        entities::{outbox_event::OutboxEvent, webhook_delivery::WebhookDelivery},
        errors::DomainError,
    },
};

// What one tick did. Returned so the worker can log it and tests can assert on it
// without reaching into repositories.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Response {
    pub events_published: usize,
    pub deliveries_created: usize,
    pub deliveries_attempted: usize,
    pub deliveries_succeeded: usize,
    pub deliveries_failed: usize,
}

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    sender: Arc<dyn WebhookSender>,
    secret_cipher: Arc<dyn SecretCipher>,
    batch_size: i64,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        sender: Arc<dyn WebhookSender>,
        secret_cipher: Arc<dyn SecretCipher>,
        batch_size: i64,
    ) -> Self {
        Self {
            unit_of_work_factory,
            sender,
            secret_cipher,
            batch_size,
        }
    }

    // One dispatcher tick. Lives in the application layer, not in the worker, so the
    // whole fan-out + retry state machine is unit-testable with in-memory fakes and
    // no HTTP; infrastructure only supplies the timer.
    pub async fn execute(&self) -> Result<Response, DomainError> {
        let mut response = Response::default();
        self.fan_out(&mut response).await?;
        self.deliver(&mut response).await?;
        Ok(response)
    }

    // Phase 1: turn each unpublished event into one delivery row per matching
    // subscription, then stamp the event published.
    //
    // Both steps commit together. If the process dies mid-phase the transaction
    // rolls back and the next tick replays it; if it dies just after commit there is
    // nothing left to replay. Either way the UNIQUE (event_id, subscription_id)
    // constraint backing `insert_many_ignoring_conflicts` means a replay can never
    // produce a duplicate delivery.
    async fn fan_out(&self, response: &mut Response) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let events = uow
            .outbox_event_repository()
            .claim_unpublished(self.batch_size)
            .await?;

        if events.is_empty() {
            // Nothing to do — drop the transaction rather than commit an empty one.
            return Ok(());
        }

        let now = Utc::now();
        let mut deliveries: Vec<WebhookDelivery> = Vec::new();
        // One event usually shares a tenant with its neighbours; cache so a batch of
        // 100 events from one tenant is a single subscription lookup.
        let mut subscriptions_by_tenant: HashMap<Uuid, Vec<_>> = HashMap::new();

        for event in &events {
            let subscriptions = match subscriptions_by_tenant.get(&event.tenant_id()) {
                Some(cached) => cached,
                None => {
                    let loaded = uow
                        .webhook_subscription_repository()
                        .find_active_by_tenant_id(event.tenant_id())
                        .await?;
                    subscriptions_by_tenant
                        .entry(event.tenant_id())
                        .or_insert(loaded)
                }
            };

            for subscription in subscriptions {
                if !subscription.accepts(event.event_type()) {
                    continue;
                }
                deliveries.push(WebhookDelivery::new(
                    event.id(),
                    subscription.id(),
                    event.tenant_id(),
                    now,
                ));
            }
        }

        uow.webhook_delivery_repository()
            .insert_many_ignoring_conflicts(&deliveries)
            .await?;

        // An event with no matching subscription is still marked published — there is
        // nobody to deliver it to, and leaving it unpublished would make the claim
        // query re-scan it forever.
        let ids: Vec<Uuid> = events.iter().map(|e| e.id()).collect();
        uow.outbox_event_repository()
            .mark_published(&ids, now)
            .await?;

        uow.commit().await?;

        response.events_published = events.len();
        response.deliveries_created = deliveries.len();
        Ok(())
    }

    // Phase 2: attempt every delivery whose backoff has elapsed.
    //
    // Each attempt is committed in its OWN transaction. That is deliberate: a batch
    // of 100 deliveries where the 50th endpoint hangs must not roll back the 49
    // results already recorded, or those events would be re-sent on the next tick.
    async fn deliver(&self, response: &mut Response) -> Result<(), DomainError> {
        let claimed = {
            let uow = self.unit_of_work_factory.begin().await?;
            let due = uow
                .webhook_delivery_repository()
                .claim_due(Utc::now(), self.batch_size)
                .await?;

            // Resolve the target and payload for each delivery while we hold the UoW.
            let mut prepared = Vec::new();
            for delivery in due {
                let subscription = uow
                    .webhook_subscription_repository()
                    .find_by_id(delivery.tenant_id(), delivery.subscription_id())
                    .await?;
                let event = uow
                    .outbox_event_repository()
                    .find_by_id(delivery.event_id())
                    .await?;

                if let (Some(subscription), Some(event)) = (subscription, event) {
                    prepared.push((delivery, subscription, event));
                }
                // A missing subscription or event means it was deleted between the
                // claim and now (ON DELETE CASCADE normally removes the delivery too).
                // Skipping leaves the row for a later tick rather than crashing.
            }
            prepared
        };

        for (mut delivery, subscription, event) in claimed {
            response.deliveries_attempted += 1;

            // Decrypt at the last possible moment, and never hold the plaintext
            // beyond this iteration.
            let secret = match self.secret_cipher.decrypt(
                subscription.secret_encrypted(),
                SecretContext::new(SecretPurpose::WebhookSecret, subscription.tenant_id()),
            ) {
                Ok(secret) => secret,
                Err(e) => {
                    // A key rotation or corrupted ciphertext. Record it as a failure so
                    // it retries and eventually dies visibly, instead of silently
                    // vanishing.
                    tracing::error!(
                        subscription_id = %subscription.id(),
                        "failed to decrypt webhook secret: {e:?}"
                    );
                    delivery.record_failure(
                        None,
                        "secret decryption failed".to_string(),
                        Utc::now(),
                    );
                    self.persist(&delivery).await?;
                    response.deliveries_failed += 1;
                    continue;
                }
            };

            let body = envelope(&event);
            let outcome = self
                .sender
                .send(WebhookRequest {
                    url: subscription.url(),
                    secret: &secret,
                    event_id: event.id(),
                    event_type: event.event_type().as_str(),
                    body,
                })
                .await;

            let now = Utc::now();
            if outcome.success {
                delivery.record_success(outcome.status.unwrap_or(200), now);
                response.deliveries_succeeded += 1;
            } else {
                delivery.record_failure(
                    outcome.status,
                    outcome
                        .error
                        .unwrap_or_else(|| "delivery failed".to_string()),
                    now,
                );
                response.deliveries_failed += 1;
            }

            self.persist(&delivery).await?;
        }

        Ok(())
    }

    async fn persist(&self, delivery: &WebhookDelivery) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        uow.webhook_delivery_repository().update(delivery).await?;
        uow.commit().await
    }
}

// Serializes the body actually sent. `id` and `occurred_at` come from the outbox
// row rather than the stored payload, so the idempotency key a consumer dedupes on
// is guaranteed to be the same value we claim in the X-Gatekeeper-Event-Id header.
pub fn envelope(event: &OutboxEvent) -> String {
    let payload = event.payload();
    let body = json!({
        "id": event.id().to_string(),
        "type": event.event_type().as_str(),
        "occurred_at": event.occurred_at().to_rfc3339(),
        "tenant_id": event.tenant_id().to_string(),
        "data": payload.get("data").cloned().unwrap_or(serde_json::Value::Null),
    });
    body.to_string()
}
