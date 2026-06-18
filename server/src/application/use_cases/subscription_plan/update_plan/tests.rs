#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::subscription_plan::update_plan::{
            command::Command, use_case::UseCase,
        },
        domain::entities::{
            billing_interval::BillingInterval, subscription_plan::SubscriptionPlan,
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

    #[tokio::test]
    async fn updates_plan_successfully() {
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let plan = SubscriptionPlan::new(
            "Basic".to_string(),
            None,
            500,
            BillingInterval::Monthly,
            1,
            10,
            true,
        );
        let plan_id = plan.id();
        plans.memory.data.lock().unwrap().insert(plan_id, plan);

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_subscription_plan_repository(plans.clone()),
        );
        let use_case = UseCase::new(uow_factory);

        let response = use_case
            .execute(Command {
                plan_id,
                name: "Basic Plus".to_string(),
                description: Some("now with more".to_string()),
                price_cents: 750,
                billing_interval: BillingInterval::Yearly,
                max_tenants: 3,
                max_users: 30,
                is_active: false,
            })
            .await
            .unwrap();

        assert_eq!(response.name, "Basic Plus");
        assert_eq!(response.price_cents, 750);
        assert_eq!(response.billing_interval, BillingInterval::Yearly);
        assert!(!response.is_active);

        let stored = plans.memory.data.lock().unwrap();
        assert_eq!(stored[&plan_id].name(), "Basic Plus");
    }
}
