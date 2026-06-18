#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::subscription::get_subscription_by_id::{
            query::Query, use_case::UseCase,
        },
        domain::{
            entities::{organization::Organization, subscription::Subscription},
            errors::DomainError,
        },
        testing::in_memory_repositories::{
            in_memory_organization_repository::InMemoryOrganizationRepository,
            in_memory_subscription_repository::InMemorySubscriptionRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
    };

    fn seed_organization(orgs: &InMemoryOrganizationRepository) -> Uuid {
        let org = Organization::new("Acme Inc".to_string(), None, true);
        let id = org.id();
        orgs.memory.data.lock().unwrap().insert(id, org);
        id
    }

    #[tokio::test]
    async fn returns_subscription_when_found() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants,
            Arc::new(InMemoryUserRepository::new()),
        ));
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let organization_id = seed_organization(&orgs);
        let sub = Subscription::new(organization_id, Uuid::now_v7(), true);
        let id = sub.id();
        subscriptions.memory.data.lock().unwrap().insert(id, sub);

        let use_case = UseCase::new(orgs, subscriptions);
        let response = use_case.execute(Query { organization_id }).await.unwrap();

        assert_eq!(response.subscription_id, id);
        assert_eq!(response.organization_id, organization_id);
    }

    #[tokio::test]
    async fn fails_when_organization_has_no_subscription() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants,
            Arc::new(InMemoryUserRepository::new()),
        ));
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let organization_id = seed_organization(&orgs);
        let use_case = UseCase::new(orgs, subscriptions);

        let err = use_case
            .execute(Query { organization_id })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_organization_does_not_exist() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants,
            Arc::new(InMemoryUserRepository::new()),
        ));
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let use_case = UseCase::new(orgs, subscriptions);

        let err = use_case
            .execute(Query {
                organization_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::OrganizationNotFound(_)));
    }
}
