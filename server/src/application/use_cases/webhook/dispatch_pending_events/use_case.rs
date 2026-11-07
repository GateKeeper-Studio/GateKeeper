use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::application::ports::secret_cipher::{SecretContext, SecretPurpose};
use crate::{
    application::{
        ports::{
            secret_cipher::SecretCipher,
            token_signer::{TokenSigner, TokenType},
            unit_of_work::{UnitOfWork, UnitOfWorkFactory},
            webhook_sender::{LogoutTokenRequest, WebhookRequest, WebhookSender},
        },
        shared::backchannel_logout::{LogoutNotice, logout_token_claims},
    },
    domain::{
        entities::{
            domain_event_type::DomainEventType,
            oauth_client::OAuthClient,
            outbox_event::OutboxEvent,
            signing_key::SigningKey,
            webhook_delivery::{DeliveryStatus, DeliveryTarget, WebhookDelivery},
            webhook_subscription::WebhookSubscription,
        },
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
    // Back-channel logout: the logout token is a JWT signed with the tenant's active
    // key and carrying the same `iss` as the ID tokens it refers to (the raw
    // OAUTH_ISSUER, exactly as TokenMinter stamps it).
    signer: Arc<dyn TokenSigner>,
    issuer: String,
    batch_size: i64,
}

// A claimed delivery with everything needed to attempt it, resolved while the claim
// transaction is open.
enum Prepared {
    Webhook {
        delivery: WebhookDelivery,
        subscription: WebhookSubscription,
        event: OutboxEvent,
    },
    BackchannelLogout {
        delivery: WebhookDelivery,
        client: OAuthClient,
        event: OutboxEvent,
        key: Option<SigningKey>,
    },
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        sender: Arc<dyn WebhookSender>,
        secret_cipher: Arc<dyn SecretCipher>,
        signer: Arc<dyn TokenSigner>,
        issuer: String,
        batch_size: i64,
    ) -> Self {
        Self {
            unit_of_work_factory,
            sender,
            secret_cipher,
            signer,
            issuer,
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
        // Back-channel logout events name their client; cache whether it still wants
        // a notification, since one sign-out everywhere is several events.
        let mut logout_targets: HashMap<(Uuid, Uuid), bool> = HashMap::new();

        for event in &events {
            if event.event_type() == DomainEventType::BackchannelLogout {
                // Not a webhook: it goes to the one OAuth client it names, if that
                // client is still active and still registers a back-channel URI.
                let Some(notice) = LogoutNotice::from_payload(event.payload()) else {
                    continue;
                };
                let key = (event.tenant_id(), notice.client_id);
                let wanted = match logout_targets.get(&key) {
                    Some(wanted) => *wanted,
                    None => {
                        let wanted = uow
                            .oauth_client_repository()
                            .find_by_id(event.tenant_id(), notice.client_id)
                            .await?
                            .is_some_and(|c| c.is_active() && c.backchannel_logout_uri().is_some());
                        logout_targets.insert(key, wanted);
                        wanted
                    }
                };
                if wanted {
                    deliveries.push(WebhookDelivery::for_backchannel_logout(
                        event.id(),
                        notice.client_id,
                        event.tenant_id(),
                        now,
                    ));
                }
                continue;
            }

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
                if let Some(ready) = Self::prepare(&*uow, delivery).await? {
                    prepared.push(ready);
                }
                // None means the target or event was deleted between the claim and
                // now (ON DELETE CASCADE normally removes the delivery too).
                // Skipping leaves the row for a later tick rather than crashing.
            }
            prepared
        };

        for ready in claimed {
            response.deliveries_attempted += 1;
            let delivery = match ready {
                Prepared::Webhook {
                    delivery,
                    subscription,
                    event,
                } => self.deliver_webhook(delivery, &subscription, &event).await,
                Prepared::BackchannelLogout {
                    delivery,
                    client,
                    event,
                    key,
                } => {
                    self.deliver_logout(delivery, &client, &event, key.as_ref())
                        .await
                }
            };

            if delivery.status() == DeliveryStatus::Delivered {
                response.deliveries_succeeded += 1;
            } else {
                response.deliveries_failed += 1;
            }
            self.persist(&delivery).await?;
        }

        Ok(())
    }

    async fn prepare(
        uow: &dyn UnitOfWork,
        delivery: WebhookDelivery,
    ) -> Result<Option<Prepared>, DomainError> {
        let Some(event) = uow
            .outbox_event_repository()
            .find_by_id(delivery.event_id())
            .await?
        else {
            return Ok(None);
        };

        Ok(match delivery.target() {
            DeliveryTarget::Subscription(subscription_id) => uow
                .webhook_subscription_repository()
                .find_by_id(delivery.tenant_id(), subscription_id)
                .await?
                .map(|subscription| Prepared::Webhook {
                    delivery,
                    subscription,
                    event,
                }),
            DeliveryTarget::BackchannelLogout(client_id) => {
                let client = uow
                    .oauth_client_repository()
                    .find_by_id(delivery.tenant_id(), client_id)
                    .await?;
                match client {
                    Some(client) => {
                        let key = uow
                            .signing_key_repository()
                            .find_active(delivery.tenant_id())
                            .await?;
                        Some(Prepared::BackchannelLogout {
                            delivery,
                            client,
                            event,
                            key,
                        })
                    }
                    None => None,
                }
            }
        })
    }

    // One webhook attempt: decrypt the subscription secret, send the HMAC-signed
    // JSON envelope, record the outcome.
    async fn deliver_webhook(
        &self,
        mut delivery: WebhookDelivery,
        subscription: &WebhookSubscription,
        event: &OutboxEvent,
    ) -> WebhookDelivery {
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
                delivery.record_failure(None, "secret decryption failed".to_string(), Utc::now());
                return delivery;
            }
        };

        let outcome = self
            .sender
            .send(WebhookRequest {
                url: subscription.url(),
                secret: &secret,
                event_id: event.id(),
                event_type: event.event_type().as_str(),
                body: envelope(event),
            })
            .await;

        record(
            &mut delivery,
            outcome.success,
            outcome.status,
            outcome.error,
        );
        delivery
    }

    // One back-channel logout attempt: mint a fresh logout token (short-lived, so
    // never reused across retries) and form-POST it to the client's URI.
    async fn deliver_logout(
        &self,
        mut delivery: WebhookDelivery,
        client: &OAuthClient,
        event: &OutboxEvent,
        key: Option<&SigningKey>,
    ) -> WebhookDelivery {
        // The client stopped wanting notifications after the event was fanned out
        // (URI removed, client deactivated). Retrying cannot fix that.
        let url = match client.backchannel_logout_uri() {
            Some(url) if client.is_active() => url,
            _ => {
                delivery.abandon("client no longer accepts back-channel logout".to_string());
                return delivery;
            }
        };
        let Some(notice) = LogoutNotice::from_payload(event.payload()) else {
            delivery.abandon("unreadable logout notice".to_string());
            return delivery;
        };
        // A tenant always has an active key; if it is momentarily missing (mid
        // rotation), retry on the normal schedule.
        let Some(key) = key else {
            delivery.record_failure(None, "no active signing key".to_string(), Utc::now());
            return delivery;
        };

        let claims = logout_token_claims(
            &self.issuer,
            &client.client_id().to_string(),
            &notice,
            Utc::now(),
        );
        let logout_token = match self.signer.sign(key, &claims, TokenType::Logout).await {
            Ok(token) => token,
            Err(e) => {
                tracing::error!(client_id = %client.id(), "failed to sign logout token: {e:?}");
                delivery.record_failure(
                    None,
                    "logout token signing failed".to_string(),
                    Utc::now(),
                );
                return delivery;
            }
        };

        let outcome = self
            .sender
            .send_logout_token(LogoutTokenRequest { url, logout_token })
            .await;

        if !outcome.success {
            tracing::warn!(
                client_id = %client.id(),
                status = ?outcome.status,
                "back-channel logout delivery failed"
            );
        }
        record(
            &mut delivery,
            outcome.success,
            outcome.status,
            outcome.error,
        );
        delivery
    }

    async fn persist(&self, delivery: &WebhookDelivery) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        uow.webhook_delivery_repository().update(delivery).await?;
        uow.commit().await
    }
}

// Applies one attempt's outcome to the retry state machine.
fn record(
    delivery: &mut WebhookDelivery,
    success: bool,
    status: Option<u16>,
    error: Option<String>,
) {
    let now = Utc::now();
    if success {
        delivery.record_success(status.unwrap_or(200), now);
    } else {
        delivery.record_failure(
            status,
            error.unwrap_or_else(|| "delivery failed".to_string()),
            now,
        );
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
