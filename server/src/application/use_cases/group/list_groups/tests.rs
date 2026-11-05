#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::group::list_groups::{query::Query, use_case::UseCase},
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

    fn seed_groups(repo: &InMemoryGroupRepository, tenant_id: Uuid, quantity: usize) {
        let mut data = repo.memory.data.lock().unwrap();
        for i in 0..quantity {
            let group = Group::new(tenant_id, format!("Group {i}"), None);
            data.insert(group.id(), group);
        }
    }

    #[tokio::test]
    async fn returns_first_page_with_defaults() {
        let tenant = make_tenant();
        let (group_repo, use_case) = setup(&tenant);
        seed_groups(&group_repo, tenant.id(), 25);

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(response.page, 1);
        assert_eq!(response.page_size, 10);
        assert_eq!(response.total, 25);
        assert_eq!(response.items.len(), 10);
    }

    #[tokio::test]
    async fn includes_member_count() {
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
        group_repo
            .members
            .lock()
            .unwrap()
            .insert((group_id, Uuid::new_v4()));
        group_repo
            .members
            .lock()
            .unwrap()
            .insert((group_id, Uuid::new_v4()));

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(response.items.len(), 1);
        assert_eq!(response.items[0].member_count, 2);
    }

    #[tokio::test]
    async fn does_not_return_groups_from_other_tenants() {
        let tenant = make_tenant();
        let other = make_tenant();
        let (group_repo, use_case) = setup(&tenant);

        seed_groups(&group_repo, tenant.id(), 3);
        seed_groups(&group_repo, other.id(), 5);

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(response.total, 3);
        assert_eq!(response.items.len(), 3);
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(Query {
                tenant_id: Uuid::new_v4(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TenantNotFound(_)));
    }
}
