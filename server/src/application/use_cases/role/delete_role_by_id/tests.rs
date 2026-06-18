#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::role::delete_role_by_id::{command::Command, use_case::UseCase},
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
            false,
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

    #[tokio::test]
    async fn deletes_role_successfully() {
        let tenant = make_tenant();
        let (role_repo, use_case) = setup(&tenant);

        let role = Role::new(tenant.id(), "Admin".into(), None);
        let role_id = role.id();
        role_repo.memory.data.lock().unwrap().insert(role_id, role);

        use_case
            .execute(Command {
                tenant_id: tenant.id(),
                role_id,
            })
            .await
            .unwrap();

        assert!(
            role_repo
                .memory
                .data
                .lock()
                .unwrap()
                .get(&role_id)
                .is_none()
        );
    }

    #[tokio::test]
    async fn fails_when_role_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                role_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::RoleNotFound(_)));
    }
}
