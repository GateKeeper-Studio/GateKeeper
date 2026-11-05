#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::role::create_role::{command::Command, use_case::UseCase},
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

    fn make_command(tenant_id: Uuid) -> Command {
        Command {
            tenant_id,
            name: "Editor".into(),
            description: Some("Content editors".into()),
        }
    }

    #[tokio::test]
    async fn creates_role_successfully() {
        let tenant = make_tenant();
        let (role_repo, use_case) = setup(&tenant);

        let response = use_case.execute(make_command(tenant.id())).await.unwrap();

        assert_eq!(response.name, "Editor");
        assert_eq!(response.description, Some("Content editors".to_string()));
        assert_eq!(response.user_count, 0);
        assert_eq!(role_repo.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn normalizes_empty_description_to_none() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let mut command = make_command(tenant.id());
        command.description = Some("   ".into());

        let response = use_case.execute(command).await.unwrap();

        assert_eq!(response.description, None);
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(make_command(Uuid::new_v4()))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TenantNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_name_already_exists() {
        let tenant = make_tenant();
        let (role_repo, use_case) = setup(&tenant);

        let existing = Role::new(tenant.id(), "Editor".into(), None);
        role_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        let err = use_case
            .execute(make_command(tenant.id()))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::RoleAlreadyExists(_)));
    }

    // A role name is privilege-bearing: it lands in the access token's `roles`
    // claim and the admin router authorizes on it. A tenant creating "admin" —
    // the most natural role name there is — would otherwise be handing its own
    // users the platform admin surface.
    #[tokio::test]
    async fn rejects_reserved_role_names() {
        let tenant = make_tenant();
        let (role_repo, use_case) = setup(&tenant);

        for name in [
            "admin",
            "tenant_admin",
            "Admin",
            "TENANT_ADMIN",
            "  admin  ",
        ] {
            let mut command = make_command(tenant.id());
            command.name = name.into();

            let err = use_case.execute(command).await.unwrap_err();

            assert!(
                matches!(err, DomainError::InvalidRequest(_)),
                "{name:?} should be rejected as reserved, got {err:?}"
            );
        }

        assert!(
            role_repo.memory.data.lock().unwrap().is_empty(),
            "no reserved role should have been persisted"
        );
    }
}
