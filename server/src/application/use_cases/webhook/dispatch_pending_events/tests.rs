#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::{
            ports::token_signer::TokenType,
            shared::backchannel_logout::{BACKCHANNEL_LOGOUT_EVENT, LogoutNotice},
            use_cases::webhook::dispatch_pending_events::use_case::UseCase,
        },
        domain::entities::{
            domain_event_type::DomainEventType,
            oauth_client::{ClientType, MfaPolicy, OAuthClient},
            outbox_event::OutboxEvent,
            signing_key::{SigningAlgorithm, SigningKey},
            webhook_delivery::{DeliveryStatus, DeliveryTarget, MAX_ATTEMPTS, WebhookDelivery},
            webhook_subscription::WebhookSubscription,
        },
        testing::{
            fake_webhook_sender::{FakeResponse, FakeWebhookSender},
            in_memory_repositories::{
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_outbox_event_repository::InMemoryOutboxEventRepository,
                in_memory_signing_key_repository::InMemorySigningKeyRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
                in_memory_webhook_delivery_repository::InMemoryWebhookDeliveryRepository,
                in_memory_webhook_subscription_repository::InMemoryWebhookSubscriptionRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            oauth_fakes::FakeTokenSigner,
            stub_secret_cipher::StubSecretCipher,
        },
    };

    struct Ctx {
        use_case: UseCase,
        events: Arc<InMemoryOutboxEventRepository>,
        subscriptions: Arc<InMemoryWebhookSubscriptionRepository>,
        deliveries: Arc<InMemoryWebhookDeliveryRepository>,
        sender: Arc<FakeWebhookSender>,
        clients: Arc<InMemoryOAuthClientRepository>,
        keys: Arc<InMemorySigningKeyRepository>,
        signer: FakeTokenSigner,
    }

    fn subscription(tenant_id: Uuid, event_types: &[&str], is_active: bool) -> WebhookSubscription {
        WebhookSubscription::new(
            tenant_id,
            "Sync".to_string(),
            "https://consumer.example.com/hook".to_string(),
            // StubSecretCipher's format, so decrypt() round-trips in the dispatcher.
            "enc:s3cret".to_string(),
            event_types.iter().map(|t| t.to_string()).collect(),
            is_active,
        )
    }

    fn event(tenant_id: Uuid, event_type: DomainEventType) -> OutboxEvent {
        OutboxEvent::new(
            tenant_id,
            event_type,
            serde_json::json!({ "type": event_type.as_str(), "data": { "user_id": "u1" } }),
        )
    }

    fn setup(sender: FakeWebhookSender) -> Ctx {
        let events = Arc::new(InMemoryOutboxEventRepository::new());
        let subscriptions = Arc::new(InMemoryWebhookSubscriptionRepository::new());
        let deliveries = Arc::new(InMemoryWebhookDeliveryRepository::new());
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        let keys = Arc::new(InMemorySigningKeyRepository::new());
        let sender = Arc::new(sender);

        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_outbox_event_repository(events.clone())
        .with_webhook_subscription_repository(subscriptions.clone())
        .with_webhook_delivery_repository(deliveries.clone())
        .with_oauth_client_repository(clients.clone())
        .with_signing_key_repository(keys.clone());

        let signer = FakeTokenSigner::new();
        let use_case = UseCase::new(
            Arc::new(factory),
            sender.clone(),
            Arc::new(StubSecretCipher),
            Arc::new(signer.clone()),
            "https://idp.example.com".to_string(),
            100,
        );

        Ctx {
            use_case,
            events,
            subscriptions,
            deliveries,
            sender,
            clients,
            keys,
            signer,
        }
    }

    fn seed_subscription(ctx: &Ctx, sub: &WebhookSubscription) {
        ctx.subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(sub.id(), sub.clone());
    }

    fn seed_event(ctx: &Ctx, event: &OutboxEvent) {
        ctx.events
            .memory
            .data
            .lock()
            .unwrap()
            .insert(event.id(), event.clone());
    }

    fn stored_deliveries(ctx: &Ctx) -> Vec<WebhookDelivery> {
        ctx.deliveries
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    #[tokio::test]
    async fn fans_out_and_delivers_a_matching_event() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        let sub = subscription(tenant_id, &["user.created"], true);
        let evt = event(tenant_id, DomainEventType::UserCreated);
        seed_subscription(&ctx, &sub);
        seed_event(&ctx, &evt);

        let result = ctx.use_case.execute().await.unwrap();

        assert_eq!(result.events_published, 1);
        assert_eq!(result.deliveries_created, 1);
        assert_eq!(result.deliveries_succeeded, 1);
        assert_eq!(result.deliveries_failed, 0);

        let stored = stored_deliveries(&ctx);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].status(), DeliveryStatus::Delivered);
        assert_eq!(stored[0].attempts(), 1);

        // The body carries the outbox id as the idempotency key, and the secret was
        // decrypted before signing.
        let sent = ctx.sender.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].event_id, evt.id());
        assert_eq!(sent[0].event_type, "user.created");
        assert_eq!(sent[0].secret, "s3cret");
        assert_eq!(sent[0].url, "https://consumer.example.com/hook");

        let body: serde_json::Value = serde_json::from_str(&sent[0].body).unwrap();
        assert_eq!(body["id"], evt.id().to_string());
        assert_eq!(body["type"], "user.created");
        assert_eq!(body["tenant_id"], tenant_id.to_string());
        assert_eq!(body["data"]["user_id"], "u1");
    }

    #[tokio::test]
    async fn an_empty_event_types_filter_means_every_event() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_id, &[], true));
        seed_event(&ctx, &event(tenant_id, DomainEventType::UserDeleted));

        let result = ctx.use_case.execute().await.unwrap();
        assert_eq!(result.deliveries_created, 1);
        assert_eq!(result.deliveries_succeeded, 1);
    }

    #[tokio::test]
    async fn skips_subscriptions_that_do_not_want_the_event_type() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_id, &["user.deleted"], true));
        seed_event(&ctx, &event(tenant_id, DomainEventType::UserCreated));

        let result = ctx.use_case.execute().await.unwrap();

        // The event is still marked published — there is nobody to send it to, and
        // leaving it pending would make the claim query rescan it forever.
        assert_eq!(result.events_published, 1);
        assert_eq!(result.deliveries_created, 0);
        assert_eq!(ctx.sender.sent_count(), 0);
    }

    #[tokio::test]
    async fn skips_inactive_subscriptions() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_id, &["user.created"], false));
        seed_event(&ctx, &event(tenant_id, DomainEventType::UserCreated));

        let result = ctx.use_case.execute().await.unwrap();
        assert_eq!(result.deliveries_created, 0);
        assert_eq!(ctx.sender.sent_count(), 0);
    }

    // Tenant isolation: one tenant's event must never reach another's endpoint.
    #[tokio::test]
    async fn never_delivers_an_event_to_another_tenants_subscription() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_a = Uuid::now_v7();
        let tenant_b = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_b, &["user.created"], true));
        seed_event(&ctx, &event(tenant_a, DomainEventType::UserCreated));

        let result = ctx.use_case.execute().await.unwrap();
        assert_eq!(result.deliveries_created, 0);
        assert_eq!(ctx.sender.sent_count(), 0);
    }

    #[tokio::test]
    async fn fans_one_event_out_to_every_matching_subscription() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_id, &["user.created"], true));
        seed_subscription(&ctx, &subscription(tenant_id, &[], true));
        seed_event(&ctx, &event(tenant_id, DomainEventType::UserCreated));

        let result = ctx.use_case.execute().await.unwrap();
        assert_eq!(result.deliveries_created, 2);
        assert_eq!(result.deliveries_succeeded, 2);
    }

    #[tokio::test]
    async fn a_rejected_delivery_schedules_a_retry_rather_than_dying() {
        let ctx = setup(FakeWebhookSender::with_default(FakeResponse::Rejected(500)));
        let tenant_id = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_id, &["user.created"], true));
        seed_event(&ctx, &event(tenant_id, DomainEventType::UserCreated));

        let result = ctx.use_case.execute().await.unwrap();
        assert_eq!(result.deliveries_failed, 1);

        let stored = stored_deliveries(&ctx);
        assert_eq!(stored[0].status(), DeliveryStatus::Pending);
        assert_eq!(stored[0].attempts(), 1);
        assert_eq!(stored[0].last_status(), Some(500));
        // Backed off, so the very next tick must not retry it.
        assert!(stored[0].next_attempt_at() > Utc::now());
    }

    #[tokio::test]
    async fn a_transport_error_is_recorded_without_a_status() {
        let ctx = setup(FakeWebhookSender::with_default(
            FakeResponse::TransportError,
        ));
        let tenant_id = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_id, &["user.created"], true));
        seed_event(&ctx, &event(tenant_id, DomainEventType::UserCreated));

        ctx.use_case.execute().await.unwrap();

        let stored = stored_deliveries(&ctx);
        assert_eq!(stored[0].status(), DeliveryStatus::Pending);
        assert!(stored[0].last_status().is_none());
        assert!(stored[0].last_error().is_some());
    }

    // A delivery that is backed off must not be claimed early.
    #[tokio::test]
    async fn does_not_claim_a_delivery_before_its_backoff_elapses() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        let sub = subscription(tenant_id, &["user.created"], true);
        let evt = event(tenant_id, DomainEventType::UserCreated);
        seed_subscription(&ctx, &sub);
        seed_event(&ctx, &evt);

        // Pre-seed a published event with a delivery scheduled an hour out.
        let mut published = evt.clone();
        published.mark_published(Utc::now());
        ctx.events
            .memory
            .data
            .lock()
            .unwrap()
            .insert(published.id(), published);

        let mut delivery = WebhookDelivery::new(evt.id(), sub.id(), tenant_id, Utc::now());
        delivery.record_failure(
            Some(500),
            "boom".to_string(),
            Utc::now() + Duration::hours(1),
        );
        ctx.deliveries
            .memory
            .data
            .lock()
            .unwrap()
            .insert(delivery.id(), delivery);

        let result = ctx.use_case.execute().await.unwrap();

        assert_eq!(result.deliveries_attempted, 0);
        assert_eq!(ctx.sender.sent_count(), 0);
    }

    // The idempotency guarantee: replaying fan-out (as happens when the worker dies
    // between inserting deliveries and stamping published_at) must not duplicate.
    #[tokio::test]
    async fn re_running_fan_out_does_not_duplicate_deliveries() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        let sub = subscription(tenant_id, &["user.created"], true);
        let evt = event(tenant_id, DomainEventType::UserCreated);
        seed_subscription(&ctx, &sub);
        seed_event(&ctx, &evt);

        // Simulate the crash: deliveries were written, the event was NOT marked
        // published, so the next tick claims it again.
        let delivery = WebhookDelivery::new(evt.id(), sub.id(), tenant_id, Utc::now());
        ctx.deliveries
            .memory
            .data
            .lock()
            .unwrap()
            .insert(delivery.id(), delivery);

        ctx.use_case.execute().await.unwrap();

        // Still exactly one delivery row, and the consumer was called once.
        assert_eq!(stored_deliveries(&ctx).len(), 1);
        assert_eq!(ctx.sender.sent_count(), 1);
    }

    // A published event is never re-fanned-out, so a consumer is not re-sent old
    // events on every tick.
    #[tokio::test]
    async fn does_not_re_publish_an_already_published_event() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_id, &["user.created"], true));

        let mut evt = event(tenant_id, DomainEventType::UserCreated);
        evt.mark_published(Utc::now());
        seed_event(&ctx, &evt);

        let result = ctx.use_case.execute().await.unwrap();
        assert_eq!(result.events_published, 0);
        assert_eq!(result.deliveries_created, 0);
    }

    #[tokio::test]
    async fn a_delivery_dies_after_max_attempts_of_repeated_failure() {
        let ctx = setup(FakeWebhookSender::with_default(FakeResponse::Rejected(503)));
        let tenant_id = Uuid::now_v7();
        let sub = subscription(tenant_id, &["user.created"], true);
        let evt = event(tenant_id, DomainEventType::UserCreated);
        seed_subscription(&ctx, &sub);
        seed_event(&ctx, &evt);

        // First tick creates the delivery and fails it once.
        ctx.use_case.execute().await.unwrap();

        // Drive the remaining attempts by forcing each one due.
        for _ in 1..MAX_ATTEMPTS {
            {
                let mut stored = ctx.deliveries.memory.data.lock().unwrap();
                for delivery in stored.values_mut() {
                    if delivery.status() == DeliveryStatus::Pending {
                        let mut refreshed = delivery.clone();
                        // Rewind the clock on the backoff so this tick claims it.
                        refreshed.record_failure(
                            Some(503),
                            "unavailable".to_string(),
                            Utc::now() - Duration::days(1),
                        );
                        *delivery = refreshed;
                    }
                }
            }
            ctx.use_case.execute().await.unwrap();
        }

        let stored = stored_deliveries(&ctx);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].status(), DeliveryStatus::Dead);
        // A dead row is never attempted again.
        let before = ctx.sender.sent_count();
        ctx.use_case.execute().await.unwrap();
        assert_eq!(ctx.sender.sent_count(), before);
    }

    #[tokio::test]
    async fn a_retry_after_a_failure_eventually_succeeds() {
        // First attempt is rejected, every later one succeeds.
        let ctx = setup(FakeWebhookSender::scripted(
            vec![FakeResponse::Rejected(500)],
            FakeResponse::Ok(200),
        ));
        let tenant_id = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_id, &["user.created"], true));
        seed_event(&ctx, &event(tenant_id, DomainEventType::UserCreated));

        ctx.use_case.execute().await.unwrap();
        assert_eq!(stored_deliveries(&ctx)[0].status(), DeliveryStatus::Pending);

        // Make the backoff elapse, then tick again.
        {
            let mut stored = ctx.deliveries.memory.data.lock().unwrap();
            for delivery in stored.values_mut() {
                let mut refreshed = delivery.clone();
                refreshed.record_failure(
                    Some(500),
                    "boom".to_string(),
                    Utc::now() - Duration::days(1),
                );
                *delivery = refreshed;
            }
        }
        ctx.use_case.execute().await.unwrap();

        let stored = stored_deliveries(&ctx);
        assert_eq!(stored[0].status(), DeliveryStatus::Delivered);
        assert!(stored[0].delivered_at().is_some());
    }

    #[tokio::test]
    async fn an_undecryptable_secret_fails_the_delivery_instead_of_the_batch() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        let mut sub = subscription(tenant_id, &["user.created"], true);
        // Not in StubSecretCipher's format — decrypt() will error.
        sub.rotate_secret("garbage-not-encrypted".to_string());
        seed_subscription(&ctx, &sub);
        seed_event(&ctx, &event(tenant_id, DomainEventType::UserCreated));

        let result = ctx.use_case.execute().await.unwrap();

        assert_eq!(result.deliveries_failed, 1);
        assert_eq!(
            ctx.sender.sent_count(),
            0,
            "must not send with a bad secret"
        );
        let stored = stored_deliveries(&ctx);
        assert_eq!(stored[0].attempts(), 1);
        assert!(stored[0].last_error().is_some());
    }

    #[tokio::test]
    async fn an_empty_outbox_is_a_no_op() {
        let ctx = setup(FakeWebhookSender::new());
        let result = ctx.use_case.execute().await.unwrap();
        assert_eq!(result, Default::default());
        assert_eq!(ctx.sender.sent_count(), 0);
    }

    // ── OIDC Back-Channel Logout ─────────────────────────────────────────────────

    const LOGOUT_URI: &str = "https://rp.example.com/backchannel-logout";

    fn seed_logout_client(ctx: &Ctx, tenant_id: Uuid, uri: Option<&str>) -> OAuthClient {
        let mut client = OAuthClient::new(
            tenant_id,
            Uuid::now_v7(),
            None,
            ClientType::Confidential,
            "Partner App".to_string(),
            vec!["https://rp.example.com/callback".to_string()],
            vec![],
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            false,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        );
        client.set_backchannel_logout(uri.map(str::to_string), false);
        ctx.clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let key = SigningKey::new(
            tenant_id,
            "kid-1".to_string(),
            SigningAlgorithm::ES256,
            "{}".to_string(),
            "secret".to_string(),
        );
        ctx.keys.memory.data.lock().unwrap().insert(key.id(), key);
        client
    }

    fn logout_event(tenant_id: Uuid, client: &OAuthClient) -> (OutboxEvent, LogoutNotice) {
        let notice = LogoutNotice {
            client_id: client.id(),
            user_id: Uuid::now_v7(),
            sid: Uuid::now_v7(),
        };
        (notice.into_event(tenant_id), notice)
    }

    #[tokio::test]
    async fn delivers_a_signed_logout_token_to_the_client_it_names() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        let client = seed_logout_client(&ctx, tenant_id, Some(LOGOUT_URI));
        let (event, notice) = logout_event(tenant_id, &client);
        seed_event(&ctx, &event);

        let result = ctx.use_case.execute().await.unwrap();

        assert_eq!(result.deliveries_created, 1);
        assert_eq!(result.deliveries_succeeded, 1);
        let sent = ctx.sender.logout_tokens();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].url, LOGOUT_URI);

        let signed = ctx.signer.signed_tokens();
        assert_eq!(signed.len(), 1);
        let (typ, claims) = &signed[0];
        assert_eq!(*typ, TokenType::Logout, "typ: logout+jwt");
        assert_eq!(claims["iss"], "https://idp.example.com");
        assert_eq!(
            claims["aud"],
            client.client_id().to_string(),
            "the PUBLIC client id"
        );
        assert_eq!(claims["sub"], notice.user_id.to_string());
        assert_eq!(claims["sid"], notice.sid.to_string());
        assert!(claims["events"].get(BACKCHANNEL_LOGOUT_EVENT).is_some());

        let stored = stored_deliveries(&ctx);
        assert_eq!(
            stored[0].target(),
            DeliveryTarget::BackchannelLogout(client.id())
        );
        assert_eq!(stored[0].status(), DeliveryStatus::Delivered);
    }

    // A subscription with an empty filter takes "every event" — but a logout
    // notification is not a webhook event and must never reach it.
    #[tokio::test]
    async fn a_logout_notice_never_reaches_a_webhook_subscription() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        seed_subscription(&ctx, &subscription(tenant_id, &[], true));
        let client = seed_logout_client(&ctx, tenant_id, Some(LOGOUT_URI));
        seed_event(&ctx, &logout_event(tenant_id, &client).0);

        ctx.use_case.execute().await.unwrap();

        assert_eq!(ctx.sender.sent_count(), 0, "no webhook");
        assert_eq!(ctx.sender.logout_tokens().len(), 1);
        assert!(
            stored_deliveries(&ctx)
                .iter()
                .all(|d| d.subscription_id().is_none())
        );
    }

    #[tokio::test]
    async fn a_client_without_a_back_channel_uri_gets_nothing() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        let client = seed_logout_client(&ctx, tenant_id, None);
        seed_event(&ctx, &logout_event(tenant_id, &client).0);

        let result = ctx.use_case.execute().await.unwrap();

        assert_eq!(
            result.events_published, 1,
            "published so it is not re-scanned"
        );
        assert_eq!(result.deliveries_created, 0);
        assert!(ctx.sender.logout_tokens().is_empty());
    }

    // The logout token lives two minutes, so a retry hours later must carry a new
    // one — never the token the first attempt sent.
    #[tokio::test]
    async fn every_attempt_mints_a_fresh_logout_token() {
        let ctx = setup(FakeWebhookSender::scripted(
            vec![FakeResponse::Rejected(400)],
            FakeResponse::Ok(200),
        ));
        let tenant_id = Uuid::now_v7();
        let client = seed_logout_client(&ctx, tenant_id, Some(LOGOUT_URI));
        seed_event(&ctx, &logout_event(tenant_id, &client).0);

        ctx.use_case.execute().await.unwrap();
        assert_eq!(stored_deliveries(&ctx)[0].status(), DeliveryStatus::Pending);

        // Make the backoff elapse, then tick again.
        {
            let mut stored = ctx.deliveries.memory.data.lock().unwrap();
            for delivery in stored.values_mut() {
                let mut due = delivery.clone();
                due.record_failure(
                    Some(400),
                    "rejected".to_string(),
                    Utc::now() - Duration::days(1),
                );
                *delivery = due;
            }
        }
        ctx.use_case.execute().await.unwrap();

        assert_eq!(
            stored_deliveries(&ctx)[0].status(),
            DeliveryStatus::Delivered
        );
        let signed = ctx.signer.signed_tokens();
        assert_eq!(signed.len(), 2, "one token per attempt");
        assert_ne!(signed[0].1["jti"], signed[1].1["jti"]);
    }

    // The client removed its URI (or was deactivated) after the notice was fanned
    // out: nothing to deliver to, so the delivery dies at once instead of retrying.
    #[tokio::test]
    async fn a_client_that_no_longer_wants_logouts_abandons_the_delivery() {
        let ctx = setup(FakeWebhookSender::new());
        let tenant_id = Uuid::now_v7();
        let client = seed_logout_client(&ctx, tenant_id, Some(LOGOUT_URI));
        let (mut event, _) = logout_event(tenant_id, &client);
        event.mark_published(Utc::now());
        seed_event(&ctx, &event);
        let delivery =
            WebhookDelivery::for_backchannel_logout(event.id(), client.id(), tenant_id, Utc::now());
        ctx.deliveries
            .memory
            .data
            .lock()
            .unwrap()
            .insert(delivery.id(), delivery);
        {
            let mut clients = ctx.clients.memory.data.lock().unwrap();
            let stored = clients.get_mut(&client.id()).unwrap();
            stored.set_backchannel_logout(None, false);
        }

        ctx.use_case.execute().await.unwrap();

        assert!(ctx.sender.logout_tokens().is_empty());
        let stored = stored_deliveries(&ctx);
        assert_eq!(stored[0].status(), DeliveryStatus::Dead);
        assert!(stored[0].last_error().unwrap().contains("no longer"));
    }
}
