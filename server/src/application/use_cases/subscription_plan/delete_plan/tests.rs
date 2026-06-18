#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::subscription_plan::delete_plan::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{billing_interval::BillingInterval, subscription_plan::SubscriptionPlan},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_subscription_plan_repository::InMemorySubscriptionPlanRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn factory(plans: Arc<InMemorySubscriptionPlanRepository>) -> Arc<InMemoryUnitOfWorkFactory> {
        Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_subscription_plan_repository(plans),
        )
    }

    #[tokio::test]
    async fn deletes_plan_successfully() {
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let plan = SubscriptionPlan::new(
            "Temp".to_string(),
            None,
            0,
            BillingInterval::Monthly,
            1,
            1,
            true,
        );
        let plan_id = plan.id();
        plans.memory.data.lock().unwrap().insert(plan_id, plan);

        let use_case = UseCase::new(factory(plans.clone()));
        use_case.execute(Command { plan_id }).await.unwrap();

        assert!(!plans.memory.data.lock().unwrap().contains_key(&plan_id));
    }

    #[tokio::test]
    async fn fails_when_not_found() {
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let use_case = UseCase::new(factory(plans));

        let err = use_case
            .execute(Command {
                plan_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionPlanNotFound(_)));
    }
}
