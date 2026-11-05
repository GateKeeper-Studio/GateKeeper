#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::group::create_group::{command::Command, use_case::UseCase},
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

    fn make_command(tenant_id: Uuid) -> Command {
        Command {
            tenant_id,
            name: "Engineering".into(),
            description: Some("The eng team".into()),
        }
    }

    #[tokio::test]
    async fn creates_group_successfully() {
        let tenant = make_tenant();
        let (group_repo, use_case) = setup(&tenant);

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap();

        assert_eq!(response.name, "Engineering");
        assert_eq!(response.description, Some("The eng team".to_string()));
        assert_eq!(response.member_count, 0);
        assert_eq!(group_repo.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn normalizes_empty_description_to_none() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let mut command = make_command(tenant.id());
        command.description = Some("   ".into());

        let response = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap();

        assert_eq!(response.description, None);
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(Uuid::new_v4()),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_name_already_exists() {
        let tenant = make_tenant();
        let (group_repo, use_case) = setup(&tenant);

        let existing = Group::new(tenant.id(), "Engineering".into(), None);
        group_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Conflict(_)));
    }

    // ── Authorization inside the use case (not only at the HTTP guard) ────────

    #[tokio::test]
    async fn an_admin_of_another_tenant_cannot_create_a_group_here() {
        let tenant = make_tenant();
        let (group_repo, use_case) = setup(&tenant);
        let intruder = crate::application::shared::actor::Actor::TenantAdmin {
            tenant_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        };

        let err = use_case
            .execute(&intruder, make_command(tenant.id()))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::AccessDenied(_)));
        assert!(group_repo.memory.data.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_end_user_cannot_use_an_admin_use_case() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);
        let user = crate::application::shared::actor::Actor::User {
            tenant_id: tenant.id(),
            user_id: Uuid::now_v7(),
        };

        let err = use_case
            .execute(&user, make_command(tenant.id()))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)));
    }

    #[tokio::test]
    async fn the_tenants_own_admin_may_create_a_group() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);
        let admin = crate::application::shared::actor::Actor::TenantAdmin {
            tenant_id: tenant.id(),
            user_id: Uuid::now_v7(),
        };
        assert!(
            use_case
                .execute(&admin, make_command(tenant.id()))
                .await
                .is_ok()
        );
    }
}
