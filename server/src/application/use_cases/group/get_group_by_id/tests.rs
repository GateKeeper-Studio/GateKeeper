#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::group::get_group_by_id::{query::Query, use_case::UseCase},
        domain::{
            entities::{group::Group, tenant::Tenant},
            errors::DomainError,
        },
        testing::in_memory_repositories::{
            in_memory_group_repository::InMemoryGroupRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
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

    fn setup(tenant: &Tenant) -> (Arc<InMemoryGroupRepository>, UseCase) {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let group_repo = Arc::new(InMemoryGroupRepository::new(user_repo));

        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let use_case = UseCase::new(group_repo.clone(), tenant_repo);
        (group_repo, use_case)
    }

    #[tokio::test]
    async fn returns_group_with_member_count() {
        let tenant = make_tenant();
        let (group_repo, use_case) = setup(&tenant);

        let group = Group::new(tenant.id(), "Eng".into(), Some("desc".into()));
        let group_id = group.id();
        group_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(group_id, group);
        group_repo
            .members
            .lock()
            .unwrap()
            .insert((group_id, Uuid::new_v4()));

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                group_id,
            })
            .await
            .unwrap();

        assert_eq!(response.name, "Eng");
        assert_eq!(response.description, Some("desc".to_string()));
        assert_eq!(response.member_count, 1);
    }

    #[tokio::test]
    async fn fails_when_group_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                group_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::GroupNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_group_belongs_to_other_tenant() {
        let tenant = make_tenant();
        let other = make_tenant();
        let (group_repo, use_case) = setup(&tenant);

        let group = Group::new(other.id(), "Eng".into(), None);
        let group_id = group.id();
        group_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(group_id, group);

        let err = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                group_id,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::GroupNotFound(_)));
    }
}
