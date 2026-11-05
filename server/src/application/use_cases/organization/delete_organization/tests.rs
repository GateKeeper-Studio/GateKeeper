#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::organization::delete_organization::{
            command::Command, use_case::UseCase,
        },
        domain::{entities::organization::Organization, errors::DomainError},
        testing::{
            in_memory_repositories::{
                in_memory_organization_repository::InMemoryOrganizationRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn factory(orgs: Arc<InMemoryOrganizationRepository>) -> Arc<InMemoryUnitOfWorkFactory> {
        Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_organization_repository(orgs),
        )
    }

    #[tokio::test]
    async fn deletes_organization_successfully() {
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let org = Organization::new("Temp".to_string(), None, true);
        let id = org.id();
        orgs.memory.data.lock().unwrap().insert(id, org);

        let use_case = UseCase::new(factory(orgs.clone()));
        use_case
            .execute(Command {
                organization_id: id,
            })
            .await
            .unwrap();

        assert!(!orgs.memory.data.lock().unwrap().contains_key(&id));
    }

    #[tokio::test]
    async fn fails_when_not_found() {
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let use_case = UseCase::new(factory(orgs));

        let err = use_case
            .execute(Command {
                organization_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
