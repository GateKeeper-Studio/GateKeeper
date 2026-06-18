#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::subscription::delete_subscription::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{organization::Organization, subscription::Subscription},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_organization_repository::InMemoryOrganizationRepository,
                in_memory_subscription_repository::InMemorySubscriptionRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn factory(
        subscriptions: Arc<InMemorySubscriptionRepository>,
        tenants: Arc<InMemoryTenantRepository>,
        orgs: Arc<InMemoryOrganizationRepository>,
    ) -> Arc<InMemoryUnitOfWorkFactory> {
        Arc::new(
            InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenants)
                .with_subscription_repository(subscriptions)
                .with_organization_repository(orgs),
        )
    }

    fn seed_organization(orgs: &InMemoryOrganizationRepository) -> Uuid {
        let org = Organization::new("Acme Inc".to_string(), None, true);
        let id = org.id();
        orgs.memory.data.lock().unwrap().insert(id, org);
        id
    }

    #[tokio::test]
    async fn deletes_subscription_successfully() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            Arc::new(InMemoryUserRepository::new()),
        ));
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let organization_id = seed_organization(&orgs);
        let sub = Subscription::new(organization_id, Uuid::now_v7(), true);
        let sub_id = sub.id();
        subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(sub_id, sub);

        let use_case = UseCase::new(factory(subscriptions.clone(), tenants, orgs));
        use_case.execute(Command { organization_id }).await.unwrap();

        assert!(
            !subscriptions
                .memory
                .data
                .lock()
                .unwrap()
                .contains_key(&sub_id)
        );
    }

    #[tokio::test]
    async fn fails_when_organization_has_no_subscription() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            Arc::new(InMemoryUserRepository::new()),
        ));
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let organization_id = seed_organization(&orgs);
        let use_case = UseCase::new(factory(subscriptions, tenants, orgs));

        let err = use_case
            .execute(Command { organization_id })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_organization_does_not_exist() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            Arc::new(InMemoryUserRepository::new()),
        ));
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let use_case = UseCase::new(factory(subscriptions, tenants, orgs));

        let err = use_case
            .execute(Command {
                organization_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::OrganizationNotFound(_)));
    }
}
