#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::webhook_subscription::create_webhook_subscription::{
            command::Command, use_case::UseCase,
        },
        domain::{entities::tenant::Tenant, errors::DomainError},
        testing::{
            in_memory_repositories::{
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
                in_memory_webhook_subscription_repository::InMemoryWebhookSubscriptionRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            stub_secret_cipher::StubSecretCipher,
        },
    };

    fn setup(
        tenant_id: Uuid,
        allow_private_targets: bool,
    ) -> (UseCase, Arc<InMemoryWebhookSubscriptionRepository>) {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "T".into(),
            None,
            "s".into(),
            true,
            true,
            true,
            Uuid::now_v7(),
            chrono::Utc::now(),
            None,
        );
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant_id, tenant);

        let subscriptions = Arc::new(InMemoryWebhookSubscriptionRepository::new());
        let factory =
            InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenants)
                .with_webhook_subscription_repository(subscriptions.clone());

        (
            UseCase::new(
                Arc::new(factory),
                Arc::new(StubSecretCipher),
                allow_private_targets,
            ),
            subscriptions,
        )
    }

    fn command(tenant_id: Uuid) -> Command {
        Command {
            tenant_id,
            name: "User sync".to_string(),
            url: "https://consumer.example.com/hook".to_string(),
            event_types: vec!["user.created".to_string()],
            is_active: true,
        }
    }

    #[tokio::test]
    async fn creates_a_subscription_and_returns_the_secret_exactly_once() {
        let tenant_id = Uuid::now_v7();
        let (use_case, subscriptions) = setup(tenant_id, false);

        let response = use_case.execute(command(tenant_id)).await.unwrap();

        assert_eq!(response.subscription.name, "User sync");
        assert_eq!(response.subscription.event_types, vec!["user.created"]);
        assert!(response.subscription.is_active);
        assert!(!response.secret.is_empty());

        // Stored encrypted, never as plaintext.
        let stored = subscriptions.memory.data.lock().unwrap();
        let saved = stored.values().next().unwrap();
        assert_ne!(saved.secret_encrypted(), response.secret);
        assert_eq!(saved.secret_encrypted(), format!("enc:{}", response.secret));
    }

    // The projection is the only shape any later read returns, so this is what
    // guarantees the secret is unrecoverable after creation.
    #[tokio::test]
    async fn the_subscription_view_never_carries_the_secret() {
        let tenant_id = Uuid::now_v7();
        let (use_case, _) = setup(tenant_id, false);

        let response = use_case.execute(command(tenant_id)).await.unwrap();
        let serialized = serde_json::to_string(&response.subscription).unwrap();

        assert!(!serialized.contains(&response.secret));
        assert!(!serialized.contains("secret"));
    }

    #[tokio::test]
    async fn each_subscription_gets_its_own_secret() {
        let tenant_id = Uuid::now_v7();
        let (use_case, _) = setup(tenant_id, false);

        let first = use_case.execute(command(tenant_id)).await.unwrap();
        let second = use_case.execute(command(tenant_id)).await.unwrap();

        assert_ne!(first.secret, second.secret);
    }

    // The SSRF boundary, exercised through the real entry point rather than only
    // against the validator in isolation.
    #[tokio::test]
    async fn refuses_a_metadata_endpoint_target() {
        let tenant_id = Uuid::now_v7();
        let (use_case, subscriptions) = setup(tenant_id, false);

        let mut cmd = command(tenant_id);
        cmd.url = "https://169.254.169.254/latest/meta-data/".to_string();

        let err = use_case.execute(cmd).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
        assert!(subscriptions.memory.data.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn refuses_a_private_range_target() {
        let tenant_id = Uuid::now_v7();
        let (use_case, _) = setup(tenant_id, false);

        let mut cmd = command(tenant_id);
        cmd.url = "https://10.0.0.5/hook".to_string();

        assert!(matches!(
            use_case.execute(cmd).await,
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[tokio::test]
    async fn allows_loopback_when_the_operator_opted_in() {
        let tenant_id = Uuid::now_v7();
        let (use_case, _) = setup(tenant_id, true);

        let mut cmd = command(tenant_id);
        cmd.url = "http://127.0.0.1:9099/hook".to_string();

        assert!(use_case.execute(cmd).await.is_ok());
    }

    #[tokio::test]
    async fn refuses_an_unknown_event_type() {
        let tenant_id = Uuid::now_v7();
        let (use_case, _) = setup(tenant_id, false);

        let mut cmd = command(tenant_id);
        cmd.event_types = vec!["user.create".to_string()];

        assert!(matches!(
            use_case.execute(cmd).await,
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[tokio::test]
    async fn an_empty_event_type_list_is_accepted_as_all_events() {
        let tenant_id = Uuid::now_v7();
        let (use_case, _) = setup(tenant_id, false);

        let mut cmd = command(tenant_id);
        cmd.event_types = vec![];

        let response = use_case.execute(cmd).await.unwrap();
        assert!(response.subscription.event_types.is_empty());
    }

    #[tokio::test]
    async fn fails_for_an_unknown_tenant() {
        let (use_case, _) = setup(Uuid::now_v7(), false);

        let err = use_case.execute(command(Uuid::now_v7())).await.unwrap_err();
        assert!(matches!(err, DomainError::TenantNotFound(_)));
    }
}
