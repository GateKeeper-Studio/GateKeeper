#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant::create_tenant::{command::Command, use_case::UseCase},
        domain::{
            entities::{
                billing_interval::BillingInterval, subscription::Subscription,
                subscription_plan::SubscriptionPlan,
            },
            errors::DomainError,
        },
        infraestructure::crypto::ecdsa_key_generator::EcdsaKeyGenerator,
        testing::{
            in_memory_repositories::{
                in_memory_signing_key_repository::InMemorySigningKeyRepository,
                in_memory_subscription_plan_repository::InMemorySubscriptionPlanRepository,
                in_memory_subscription_repository::InMemorySubscriptionRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    // ── Helpers ────────────────────────────────────────────────────────────────

    // Returns the repos, the seeded subscription id (with a generous plan), and the
    // use case.
    fn setup() -> (
        Arc<InMemoryTenantRepository>,
        Arc<InMemorySigningKeyRepository>,
        Uuid,
        UseCase,
    ) {
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let signing_keys = Arc::new(InMemorySigningKeyRepository::new());
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenant_repo.clone(),
            Arc::new(InMemoryUserRepository::new()),
        ));

        let plan = SubscriptionPlan::new(
            "Test Plan".to_string(),
            None,
            0,
            BillingInterval::Monthly,
            100,
            1000,
            true,
        );
        let subscription = Subscription::new(uuid::Uuid::now_v7(), plan.id(), true);
        let subscription_id = subscription.id();
        plans.memory.data.lock().unwrap().insert(plan.id(), plan);
        subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(subscription_id, subscription);

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                tenant_repo.clone(),
            )
            .with_signing_key_repository(signing_keys.clone())
            .with_subscription_repository(subscriptions.clone())
            .with_subscription_plan_repository(plans.clone()),
        );
        let use_case = UseCase::new(uow_factory, Arc::new(EcdsaKeyGenerator::new()));
        (tenant_repo, signing_keys, subscription_id, use_case)
    }

    fn make_command(name: &str, subscription_id: Uuid) -> Command {
        Command {
            subscription_id,
            name: name.to_string(),
            description: Some("A test tenant".to_string()),
            password_hash_secret: "super_secret_123".to_string(),
            can_self_register: true,
            can_self_forgot_password: true,
        }
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn creates_tenant_successfully() {
        let (tenant_repo, signing_keys, subscription_id, use_case) = setup();

        let response = use_case
            .execute(make_command("Acme Corp", subscription_id))
            .await
            .unwrap();

        assert_eq!(response.name, "Acme Corp");
        assert_eq!(response.description, Some("A test tenant".to_string()));
        assert!(response.is_active);
        assert!(response.can_self_register);
        assert!(response.can_self_forgot_password);
        assert_eq!(response.subscription_id, subscription_id);
        assert!(!response.is_default);
        assert_eq!(tenant_repo.memory.data.lock().unwrap().len(), 1);

        // A signing key is provisioned for the new tenant in the same transaction.
        let keys = signing_keys.memory.data.lock().unwrap();
        assert_eq!(keys.len(), 1);
        let key = keys.values().next().unwrap();
        assert_eq!(key.tenant_id(), response.tenant_id);
        assert!(key.is_active());
    }

    #[tokio::test]
    async fn creates_tenant_without_description() {
        let (_, _, subscription_id, use_case) = setup();

        let mut command = make_command("Minimal Corp", subscription_id);
        command.description = None;

        let response = use_case.execute(command).await.unwrap();

        assert_eq!(response.description, None);
    }

    #[tokio::test]
    async fn fails_when_subscription_missing() {
        let (_, _, _, use_case) = setup();

        let err = use_case
            .execute(make_command("Orphan Corp", Uuid::now_v7()))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_tenant_name_already_exists() {
        let (tenant_repo, _, subscription_id, use_case) = setup();

        // Seed a tenant with the same name directly into committed data.
        use crate::domain::entities::tenant::Tenant;
        let existing = Tenant::new(
            "Duplicate Name".to_string(),
            None,
            "secret".to_string(),
            true,
            false,
            false,
            uuid::Uuid::now_v7(),
            false,
        );
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        let err = use_case
            .execute(make_command("Duplicate Name", subscription_id))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TenantAlreadyExists(_)));
    }

    #[tokio::test]
    async fn persists_tenant_to_repository() {
        let (tenant_repo, _, subscription_id, use_case) = setup();

        let response = use_case
            .execute(make_command("Persist Corp", subscription_id))
            .await
            .unwrap();

        let saved = tenant_repo.memory.data.lock().unwrap();
        assert!(saved.contains_key(&response.tenant_id));
        assert_eq!(saved[&response.tenant_id].name(), "Persist Corp");
    }
}
