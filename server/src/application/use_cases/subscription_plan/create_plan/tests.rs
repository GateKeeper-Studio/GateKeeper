#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::subscription_plan::create_plan::{
            command::Command, use_case::UseCase,
        },
        domain::{entities::billing_interval::BillingInterval, errors::DomainError},
        testing::{
            in_memory_repositories::{
                in_memory_subscription_plan_repository::InMemorySubscriptionPlanRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn setup() -> (Arc<InMemorySubscriptionPlanRepository>, UseCase) {
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_subscription_plan_repository(plans.clone()),
        );
        (plans, UseCase::new(uow_factory))
    }

    fn make_command(name: &str) -> Command {
        Command {
            name: name.to_string(),
            description: Some("A test plan".to_string()),
            price_cents: 1999,
            billing_interval: BillingInterval::Monthly,
            max_tenants: 5,
            max_users: 100,
            is_active: true,
        }
    }

    #[tokio::test]
    async fn creates_plan_successfully() {
        let (plans, use_case) = setup();

        let response = use_case.execute(make_command("Pro")).await.unwrap();

        assert_eq!(response.name, "Pro");
        assert_eq!(response.price_cents, 1999);
        assert_eq!(response.billing_interval, BillingInterval::Monthly);
        assert_eq!(plans.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn fails_when_plan_name_already_exists() {
        let (plans, use_case) = setup();

        use crate::domain::entities::subscription_plan::SubscriptionPlan;
        let existing = SubscriptionPlan::new(
            "Duplicate".to_string(),
            None,
            0,
            BillingInterval::Yearly,
            1,
            1,
            true,
        );
        plans
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        let err = use_case
            .execute(make_command("Duplicate"))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionPlanAlreadyExists(_)));
    }
}
