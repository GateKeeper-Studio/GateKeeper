#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::subscription_plan::list_plans::{query::Query, use_case::UseCase},
        domain::entities::{
            billing_interval::BillingInterval, subscription_plan::SubscriptionPlan,
        },
        testing::in_memory_repositories::in_memory_subscription_plan_repository::InMemorySubscriptionPlanRepository,
    };

    #[tokio::test]
    async fn lists_plans_with_pagination() {
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        {
            let mut data = plans.memory.data.lock().unwrap();
            for i in 0..3 {
                let plan = SubscriptionPlan::new(
                    format!("Plan {i}"),
                    None,
                    (i as i64) * 100,
                    BillingInterval::Monthly,
                    1,
                    10,
                    true,
                );
                data.insert(plan.id(), plan);
            }
        }

        let use_case = UseCase::new(plans);
        let result = use_case
            .execute(Query {
                page: Some(1),
                page_size: Some(10),
            })
            .await
            .unwrap();

        assert_eq!(result.total, 3);
        assert_eq!(result.items.len(), 3);
    }
}
