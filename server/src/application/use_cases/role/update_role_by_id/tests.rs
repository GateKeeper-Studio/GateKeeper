#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::role::update_role_by_id::{command::Command, use_case::UseCase},
        domain::{
            entities::{role::Role, tenant::Tenant},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_role_repository::InMemoryRoleRepository,
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

    fn setup(tenant: &Tenant) -> (Arc<InMemoryRoleRepository>, UseCase) {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let role_repo = Arc::new(InMemoryRoleRepository::new(user_repo.clone()));

        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(user_repo, tenant_repo)
                .with_role_repository(role_repo.clone()),
        );

        (role_repo, UseCase::new(uow_factory))
    }

    fn seed_role(repo: &InMemoryRoleRepository, tenant_id: Uuid, name: &str) -> Uuid {
        let role = Role::new(tenant_id, name.into(), None);
        let id = role.id();
        repo.memory.data.lock().unwrap().insert(id, role);
        id
    }

    #[tokio::test]
    async fn updates_role_successfully() {
        let tenant = make_tenant();
        let (role_repo, use_case) = setup(&tenant);
        let role_id = seed_role(&role_repo, tenant.id(), "Old");

        let response = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                role_id,
                name: "New".into(),
                description: Some("desc".into()),
            })
            .await
            .unwrap();

        assert_eq!(response.name, "New");
        assert_eq!(response.description, Some("desc".to_string()));
        assert!(response.updated_at.is_some());

        let saved = role_repo
            .memory
            .data
            .lock()
            .unwrap()
            .get(&role_id)
            .cloned()
            .unwrap();
        assert_eq!(saved.name(), "New");
    }

    #[tokio::test]
    async fn fails_when_role_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                role_id: Uuid::new_v4(),
                name: "New".into(),
                description: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::RoleNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_renaming_to_existing_name() {
        let tenant = make_tenant();
        let (role_repo, use_case) = setup(&tenant);
        seed_role(&role_repo, tenant.id(), "Taken");
        let role_id = seed_role(&role_repo, tenant.id(), "Original");

        let err = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                role_id,
                name: "Taken".into(),
                description: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::RoleAlreadyExists(_)));
    }

    #[tokio::test]
    async fn allows_keeping_same_name() {
        let tenant = make_tenant();
        let (role_repo, use_case) = setup(&tenant);
        let role_id = seed_role(&role_repo, tenant.id(), "Same");

        let response = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                role_id,
                name: "Same".into(),
                description: Some("now with desc".into()),
            })
            .await
            .unwrap();

        assert_eq!(response.name, "Same");
        assert_eq!(response.description, Some("now with desc".to_string()));
    }

    // Renaming into a reserved name is the obvious way around the check on create,
    // so it is refused here too. See `create_role::rejects_reserved_role_names`.
    #[tokio::test]
    async fn rejects_renaming_to_a_reserved_name() {
        let tenant = make_tenant();
        let (role_repo, use_case) = setup(&tenant);
        let role_id = seed_role(&role_repo, tenant.id(), "Original");

        for name in ["admin", "tenant_admin", "Admin"] {
            let err = use_case
                .execute(Command {
                    tenant_id: tenant.id(),
                    role_id,
                    name: name.into(),
                    description: None,
                })
                .await
                .unwrap_err();

            assert!(
                matches!(err, DomainError::InvalidRequest(_)),
                "{name:?} should be rejected as reserved, got {err:?}"
            );
        }

        let saved = role_repo.memory.data.lock().unwrap();
        assert_eq!(saved.get(&role_id).unwrap().name(), "Original");
    }
}
