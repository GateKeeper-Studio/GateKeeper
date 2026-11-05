#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::role::assign_role_users::{command::Command, use_case::UseCase},
        domain::{
            entities::{role::Role, tenant::Tenant},
            errors::DomainError,
            repositories::role_repository::RoleRepository,
        },
        testing::{
            in_memory_repositories::{
                in_memory_role_repository::InMemoryRoleRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            user_factory::generate_user,
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

    fn setup(
        tenant: &Tenant,
    ) -> (
        Arc<InMemoryUserRepository>,
        Arc<InMemoryRoleRepository>,
        UseCase,
    ) {
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
            InMemoryUnitOfWorkFactory::new(user_repo.clone(), tenant_repo)
                .with_role_repository(role_repo.clone()),
        );

        (user_repo, role_repo, UseCase::new(uow_factory))
    }

    fn seed_role(repo: &InMemoryRoleRepository, tenant_id: Uuid) -> Uuid {
        let role = Role::new(tenant_id, "Admin".into(), None);
        let id = role.id();
        repo.memory.data.lock().unwrap().insert(id, role);
        id
    }

    fn seed_user(repo: &InMemoryUserRepository, tenant_id: Uuid) -> Uuid {
        let user = generate_user(tenant_id);
        let id = user.id();
        repo.memory.data.lock().unwrap().insert(id, user);
        id
    }

    #[tokio::test]
    async fn assigns_users_successfully() {
        let tenant = make_tenant();
        let (user_repo, role_repo, use_case) = setup(&tenant);
        let role_id = seed_role(&role_repo, tenant.id());
        let u1 = seed_user(&user_repo, tenant.id());
        let u2 = seed_user(&user_repo, tenant.id());

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    role_id,
                    user_ids: vec![u1, u2],
                },
            )
            .await
            .unwrap();

        assert_eq!(response.assigned, 2);
        assert_eq!(response.user_count, 2);
        assert!(role_repo.is_user_assigned(role_id, u1).await.unwrap());
        assert!(role_repo.is_user_assigned(role_id, u2).await.unwrap());
    }

    #[tokio::test]
    async fn is_idempotent_for_existing_assignment() {
        let tenant = make_tenant();
        let (user_repo, role_repo, use_case) = setup(&tenant);
        let role_id = seed_role(&role_repo, tenant.id());
        let u1 = seed_user(&user_repo, tenant.id());
        role_repo.assignments.lock().unwrap().insert((role_id, u1));

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    role_id,
                    user_ids: vec![u1],
                },
            )
            .await
            .unwrap();

        assert_eq!(response.user_count, 1);
    }

    #[tokio::test]
    async fn fails_when_user_belongs_to_other_tenant() {
        let tenant = make_tenant();
        let other = make_tenant();
        let (user_repo, role_repo, use_case) = setup(&tenant);
        let role_id = seed_role(&role_repo, tenant.id());
        let foreign_user = seed_user(&user_repo, other.id());

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    role_id,
                    user_ids: vec![foreign_user],
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_role_not_found() {
        let tenant = make_tenant();
        let (user_repo, _role_repo, use_case) = setup(&tenant);
        let u1 = seed_user(&user_repo, tenant.id());

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    role_id: Uuid::new_v4(),
                    user_ids: vec![u1],
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
