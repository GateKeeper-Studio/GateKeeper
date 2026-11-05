#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::organization::create_organization::{
            command::Command, use_case::UseCase,
        },
        domain::errors::DomainError,
        testing::{
            in_memory_repositories::{
                in_memory_organization_repository::InMemoryOrganizationRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn setup() -> (Arc<InMemoryOrganizationRepository>, UseCase) {
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_organization_repository(orgs.clone()),
        );
        (orgs, UseCase::new(uow_factory))
    }

    fn make_command(name: &str) -> Command {
        Command {
            name: name.to_string(),
            description: Some("a test org".to_string()),
            is_active: true,
        }
    }

    #[tokio::test]
    async fn creates_organization_successfully() {
        let (orgs, use_case) = setup();

        let response = use_case.execute(make_command("Acme Inc")).await.unwrap();

        assert_eq!(response.name, "Acme Inc");
        assert!(response.is_active);
        assert_eq!(orgs.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn fails_when_name_already_exists() {
        let (orgs, use_case) = setup();

        use crate::domain::entities::organization::Organization;
        let existing = Organization::new("Duplicate".to_string(), None, true);
        orgs.memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        let err = use_case
            .execute(make_command("Duplicate"))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Conflict(_)));
    }
}
