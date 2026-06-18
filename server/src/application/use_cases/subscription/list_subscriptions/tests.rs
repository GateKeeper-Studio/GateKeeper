#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::subscription::list_subscriptions::{
            query::Query, use_case::UseCase,
        },
        domain::entities::subscription::Subscription,
        testing::in_memory_repositories::{
            in_memory_subscription_repository::InMemorySubscriptionRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
    };

    #[tokio::test]
    async fn lists_subscriptions() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants,
            Arc::new(InMemoryUserRepository::new()),
        ));
        {
            let mut data = subscriptions.memory.data.lock().unwrap();
            for _ in 0..2 {
                let sub = Subscription::new(uuid::Uuid::now_v7(), Uuid::now_v7(), true);
                data.insert(sub.id(), sub);
            }
        }

        let use_case = UseCase::new(subscriptions);
        let result = use_case
            .execute(Query {
                page: Some(1),
                page_size: Some(10),
            })
            .await
            .unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);
    }
}
