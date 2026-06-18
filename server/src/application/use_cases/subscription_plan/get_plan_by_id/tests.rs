#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::subscription_plan::get_plan_by_id::{
            query::Query, use_case::UseCase,
        },
        domain::{
            entities::{billing_interval::BillingInterval, subscription_plan::SubscriptionPlan},
            errors::DomainError,
        },
        testing::in_memory_repositories::in_memory_subscription_plan_repository::InMemorySubscriptionPlanRepository,
    };

    #[tokio::test]
    async fn returns_plan_when_found() {
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let plan = SubscriptionPlan::new(
            "Pro".to_string(),
            None,
            999,
            BillingInterval::Monthly,
            1,
            10,
            true,
        );
        let plan_id = plan.id();
        plans.memory.data.lock().unwrap().insert(plan_id, plan);

        let use_case = UseCase::new(plans);
        let response = use_case.execute(Query { plan_id }).await.unwrap();

        assert_eq!(response.plan_id, plan_id);
        assert_eq!(response.name, "Pro");
    }

    #[tokio::test]
    async fn fails_when_not_found() {
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let use_case = UseCase::new(plans);

        let err = use_case
            .execute(Query {
                plan_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionPlanNotFound(_)));
    }
}
