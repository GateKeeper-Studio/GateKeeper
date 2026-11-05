#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::group::update_group_by_id::{command::Command, use_case::UseCase},
        domain::{
            entities::{group::Group, tenant::Tenant},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_group_repository::InMemoryGroupRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn make_tenant() -> Tenant {
        Tenant::new(
            "Test Tenant".into(),
            None,
            "secret".into(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
        )
    }

    fn setup(tenant: &Tenant) -> (Arc<InMemoryGroupRepository>, UseCase) {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let group_repo = Arc::new(InMemoryGroupRepository::new(user_repo.clone()));

        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(user_repo, tenant_repo)
                .with_group_repository(group_repo.clone()),
        );

        (group_repo, UseCase::new(uow_factory))
    }

    fn seed_group(repo: &InMemoryGroupRepository, tenant_id: Uuid, name: &str) -> Uuid {
        let group = Group::new(tenant_id, name.into(), None);
        let id = group.id();
        repo.memory.data.lock().unwrap().insert(id, group);
        id
    }

    #[tokio::test]
    async fn updates_group_successfully() {
        let tenant = make_tenant();
        let (group_repo, use_case) = setup(&tenant);
        let group_id = seed_group(&group_repo, tenant.id(), "Old");

        let response = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                group_id,
                name: "New".into(),
                description: Some("desc".into()),
            })
            .await
            .unwrap();

        assert_eq!(response.name, "New");
        assert_eq!(response.description, Some("desc".to_string()));
        assert!(response.updated_at.is_some());

        let saved = group_repo
            .memory
            .data
            .lock()
            .unwrap()
            .get(&group_id)
            .cloned()
            .unwrap();
        assert_eq!(saved.name(), "New");
    }

    #[tokio::test]
    async fn fails_when_group_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                group_id: Uuid::new_v4(),
                name: "New".into(),
                description: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::GroupNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_renaming_to_existing_name() {
        let tenant = make_tenant();
        let (group_repo, use_case) = setup(&tenant);
        seed_group(&group_repo, tenant.id(), "Taken");
        let group_id = seed_group(&group_repo, tenant.id(), "Original");

        let err = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                group_id,
                name: "Taken".into(),
                description: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::GroupAlreadyExists(_)));
    }

    #[tokio::test]
    async fn allows_keeping_same_name() {
        let tenant = make_tenant();
        let (group_repo, use_case) = setup(&tenant);
        let group_id = seed_group(&group_repo, tenant.id(), "Same");

        let response = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                group_id,
                name: "Same".into(),
                description: Some("now with desc".into()),
            })
            .await
            .unwrap();

        assert_eq!(response.name, "Same");
        assert_eq!(response.description, Some("now with desc".to_string()));
    }
}
