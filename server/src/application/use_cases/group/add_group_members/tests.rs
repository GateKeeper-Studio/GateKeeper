#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::group::add_group_members::{command::Command, use_case::UseCase},
        domain::{
            entities::{group::Group, tenant::Tenant},
            errors::DomainError,
            repositories::group_repository::GroupRepository,
        },
        testing::{
            in_memory_repositories::{
                in_memory_group_repository::InMemoryGroupRepository,
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
        Arc<InMemoryGroupRepository>,
        UseCase,
    ) {
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
            InMemoryUnitOfWorkFactory::new(user_repo.clone(), tenant_repo)
                .with_group_repository(group_repo.clone()),
        );

        (user_repo, group_repo, UseCase::new(uow_factory))
    }

    fn seed_group(repo: &InMemoryGroupRepository, tenant_id: Uuid) -> Uuid {
        let group = Group::new(tenant_id, "Eng".into(), None);
        let id = group.id();
        repo.memory.data.lock().unwrap().insert(id, group);
        id
    }

    fn seed_user(repo: &InMemoryUserRepository, tenant_id: Uuid) -> Uuid {
        let user = generate_user(tenant_id);
        let id = user.id();
        repo.memory.data.lock().unwrap().insert(id, user);
        id
    }

    #[tokio::test]
    async fn adds_members_successfully() {
        let tenant = make_tenant();
        let (user_repo, group_repo, use_case) = setup(&tenant);
        let group_id = seed_group(&group_repo, tenant.id());
        let u1 = seed_user(&user_repo, tenant.id());
        let u2 = seed_user(&user_repo, tenant.id());

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    group_id,
                    user_ids: vec![u1, u2],
                },
            )
            .await
            .unwrap();

        assert_eq!(response.added, 2);
        assert_eq!(response.member_count, 2);
        assert!(group_repo.is_member(group_id, u1).await.unwrap());
        assert!(group_repo.is_member(group_id, u2).await.unwrap());
    }

    #[tokio::test]
    async fn is_idempotent_for_existing_member() {
        let tenant = make_tenant();
        let (user_repo, group_repo, use_case) = setup(&tenant);
        let group_id = seed_group(&group_repo, tenant.id());
        let u1 = seed_user(&user_repo, tenant.id());
        group_repo.members.lock().unwrap().insert((group_id, u1));

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    group_id,
                    user_ids: vec![u1],
                },
            )
            .await
            .unwrap();

        assert_eq!(response.member_count, 1);
    }

    #[tokio::test]
    async fn fails_when_user_belongs_to_other_tenant() {
        let tenant = make_tenant();
        let other = make_tenant();
        let (user_repo, group_repo, use_case) = setup(&tenant);
        let group_id = seed_group(&group_repo, tenant.id());
        let foreign_user = seed_user(&user_repo, other.id());

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    group_id,
                    user_ids: vec![foreign_user],
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_group_not_found() {
        let tenant = make_tenant();
        let (user_repo, _group_repo, use_case) = setup(&tenant);
        let u1 = seed_user(&user_repo, tenant.id());

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    group_id: Uuid::new_v4(),
                    user_ids: vec![u1],
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
