#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::role::get_role_by_id::{query::Query, use_case::UseCase},
        domain::{
            entities::{role::Role, tenant::Tenant},
            errors::DomainError,
        },
        testing::in_memory_repositories::{
            in_memory_role_repository::InMemoryRoleRepository,
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

    fn setup(tenant: &Tenant) -> (Arc<InMemoryRoleRepository>, UseCase) {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let role_repo = Arc::new(InMemoryRoleRepository::new(user_repo));

        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let use_case = UseCase::new(role_repo.clone(), tenant_repo);
        (role_repo, use_case)
    }

    #[tokio::test]
    async fn returns_role_with_user_count() {
        let tenant = make_tenant();
        let (role_repo, use_case) = setup(&tenant);

        let role = Role::new(tenant.id(), "Admin".into(), Some("desc".into()));
        let role_id = role.id();
        role_repo.memory.data.lock().unwrap().insert(role_id, role);
        role_repo
            .assignments
            .lock()
            .unwrap()
            .insert((role_id, Uuid::new_v4()));

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                role_id,
            })
            .await
            .unwrap();

        assert_eq!(response.name, "Admin");
        assert_eq!(response.description, Some("desc".to_string()));
        assert_eq!(response.user_count, 1);
    }

    #[tokio::test]
    async fn fails_when_role_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                role_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::RoleNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_role_belongs_to_other_tenant() {
        let tenant = make_tenant();
        let other = make_tenant();
        let (role_repo, use_case) = setup(&tenant);

        let role = Role::new(other.id(), "Admin".into(), None);
        let role_id = role.id();
        role_repo.memory.data.lock().unwrap().insert(role_id, role);

        let err = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                role_id,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::RoleNotFound(_)));
    }
}
