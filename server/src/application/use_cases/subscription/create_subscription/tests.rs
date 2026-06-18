#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::subscription::create_subscription::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                billing_interval::BillingInterval, organization::Organization,
                subscription::Subscription, subscription_plan::SubscriptionPlan,
            },
            errors::DomainError,
        },
        infraestructure::crypto::ecdsa_key_generator::EcdsaKeyGenerator,
        testing::{
            in_memory_repositories::{
                in_memory_organization_repository::InMemoryOrganizationRepository,
                in_memory_signing_key_repository::InMemorySigningKeyRepository,
                in_memory_subscription_plan_repository::InMemorySubscriptionPlanRepository,
                in_memory_subscription_repository::InMemorySubscriptionRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    struct Ctx {
        tenants: Arc<InMemoryTenantRepository>,
        subscriptions: Arc<InMemorySubscriptionRepository>,
        plans: Arc<InMemorySubscriptionPlanRepository>,
        signing_keys: Arc<InMemorySigningKeyRepository>,
        organization_id: Uuid,
        use_case: UseCase,
    }

    fn setup() -> Ctx {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            Arc::new(InMemoryUserRepository::new()),
        ));
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let signing_keys = Arc::new(InMemorySigningKeyRepository::new());

        // Seed an organization that the subscription will belong to.
        let org = Organization::new("Acme Inc".to_string(), None, true);
        let organization_id = org.id();
        orgs.memory
            .data
            .lock()
            .unwrap()
            .insert(organization_id, org);

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                tenants.clone(),
            )
            .with_subscription_repository(subscriptions.clone())
            .with_subscription_plan_repository(plans.clone())
            .with_organization_repository(orgs.clone())
            .with_signing_key_repository(signing_keys.clone()),
        );

        let use_case = UseCase::new(uow_factory, Arc::new(EcdsaKeyGenerator::new()));
        Ctx {
            tenants,
            subscriptions,
            plans,
            signing_keys,
            organization_id,
            use_case,
        }
    }

    fn seed_plan(plans: &InMemorySubscriptionPlanRepository) -> Uuid {
        let plan = SubscriptionPlan::new(
            "Pro".to_string(),
            None,
            0,
            BillingInterval::Monthly,
            5,
            100,
            true,
        );
        let id = plan.id();
        plans.memory.data.lock().unwrap().insert(id, plan);
        id
    }

    #[tokio::test]
    async fn creates_subscription_with_master_tenant_and_signing_key() {
        let ctx = setup();
        let plan_id = seed_plan(&ctx.plans);

        let response = ctx
            .use_case
            .execute(Command {
                organization_id: ctx.organization_id,
                plan_id,
            })
            .await
            .unwrap();

        assert_eq!(response.plan_id, plan_id);
        assert_eq!(response.organization_id, ctx.organization_id);

        // Subscription persisted.
        assert_eq!(ctx.subscriptions.memory.data.lock().unwrap().len(), 1);

        // Exactly one default "master" tenant for this subscription.
        let tenants = ctx.tenants.memory.data.lock().unwrap();
        assert_eq!(tenants.len(), 1);
        let master = tenants.get(&response.default_tenant_id).unwrap();
        assert_eq!(master.name(), "master");
        assert!(master.is_default());
        assert_eq!(master.subscription_id(), response.subscription_id);

        // Signing key provisioned for the master tenant.
        let keys = ctx.signing_keys.memory.data.lock().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(
            keys.values().next().unwrap().tenant_id(),
            response.default_tenant_id
        );
    }

    #[tokio::test]
    async fn fails_when_organization_does_not_exist() {
        let ctx = setup();
        let plan_id = seed_plan(&ctx.plans);

        let err = ctx
            .use_case
            .execute(Command {
                organization_id: Uuid::now_v7(),
                plan_id,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::OrganizationNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_organization_already_has_subscription() {
        let ctx = setup();
        let plan_id = seed_plan(&ctx.plans);

        // Seed an existing subscription for the same organization.
        let existing = Subscription::new(ctx.organization_id, plan_id, true);
        ctx.subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        let err = ctx
            .use_case
            .execute(Command {
                organization_id: ctx.organization_id,
                plan_id,
            })
            .await
            .unwrap_err();

        assert!(matches!(
            err,
            DomainError::OrganizationAlreadyHasSubscription(_)
        ));
    }

    #[tokio::test]
    async fn fails_when_plan_does_not_exist() {
        let ctx = setup();

        let err = ctx
            .use_case
            .execute(Command {
                organization_id: ctx.organization_id,
                plan_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionPlanNotFound(_)));
        // Nothing committed.
        assert_eq!(ctx.subscriptions.memory.data.lock().unwrap().len(), 0);
        assert_eq!(ctx.tenants.memory.data.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn fails_when_plan_is_inactive() {
        let ctx = setup();
        let plan = SubscriptionPlan::new(
            "Inactive".to_string(),
            None,
            0,
            BillingInterval::Monthly,
            5,
            100,
            false,
        );
        let plan_id = plan.id();
        ctx.plans.memory.data.lock().unwrap().insert(plan_id, plan);

        let err = ctx
            .use_case
            .execute(Command {
                organization_id: ctx.organization_id,
                plan_id,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionPlanInactive(_)));
        assert_eq!(ctx.subscriptions.memory.data.lock().unwrap().len(), 0);
        assert_eq!(ctx.tenants.memory.data.lock().unwrap().len(), 0);
    }
}
