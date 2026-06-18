#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::organization::update_organization::{
            command::Command, use_case::UseCase,
        },
        domain::entities::organization::Organization,
        testing::{
            in_memory_repositories::{
                in_memory_organization_repository::InMemoryOrganizationRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    #[tokio::test]
    async fn updates_organization_successfully() {
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let org = Organization::new("Old".to_string(), None, true);
        let id = org.id();
        orgs.memory.data.lock().unwrap().insert(id, org);

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_organization_repository(orgs.clone()),
        );
        let use_case = UseCase::new(uow_factory);

        let response = use_case
            .execute(Command {
                organization_id: id,
                name: "New".to_string(),
                description: Some("renamed".to_string()),
                is_active: false,
            })
            .await
            .unwrap();

        assert_eq!(response.name, "New");
        assert!(!response.is_active);
        assert_eq!(orgs.memory.data.lock().unwrap()[&id].name(), "New");
    }
}
