#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::group::delete_group_by_id::{command::Command, use_case::UseCase},
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

    #[tokio::test]
    async fn deletes_group_successfully() {
        let tenant = make_tenant();
        let (group_repo, use_case) = setup(&tenant);

        let group = Group::new(tenant.id(), "Eng".into(), None);
        let group_id = group.id();
        group_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(group_id, group);

        use_case
            .execute(Command {
                tenant_id: tenant.id(),
                group_id,
            })
            .await
            .unwrap();

        assert!(
            group_repo
                .memory
                .data
                .lock()
                .unwrap()
                .get(&group_id)
                .is_none()
        );
    }

    #[tokio::test]
    async fn fails_when_group_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                group_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::GroupNotFound(_)));
    }
}
