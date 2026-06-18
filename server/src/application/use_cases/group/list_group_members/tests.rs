#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::group::list_group_members::{query::Query, use_case::UseCase},
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
            false,
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

        let use_case = UseCase::new(group_repo.clone(), tenant_repo);
        (user_repo, group_repo, use_case)
    }

    #[tokio::test]
    async fn lists_members_of_group() {
        let tenant = make_tenant();
        let (user_repo, group_repo, use_case) = setup(&tenant);

        let group = Group::new(tenant.id(), "Eng".into(), None);
        let group_id = group.id();
        group_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(group_id, group);

        for _ in 0..3 {
            let user = generate_user(tenant.id());
            let user_id = user.id();
            user_repo.memory.data.lock().unwrap().insert(user_id, user);
            group_repo
                .members
                .lock()
                .unwrap()
                .insert((group_id, user_id));
        }
        // Noise: a user not in the group.
        let other = generate_user(tenant.id());
        user_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(other.id(), other);

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                group_id,
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(response.total, 3);
        assert_eq!(response.items.len(), 3);
    }

    #[tokio::test]
    async fn fails_when_group_not_found() {
        let tenant = make_tenant();
        let (_, _, use_case) = setup(&tenant);

        let err = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                group_id: Uuid::new_v4(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::GroupNotFound(_)));
    }
}
