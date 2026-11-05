#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::role::list_role_users::{query::Query, use_case::UseCase},
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

        let use_case = UseCase::new(role_repo.clone(), tenant_repo);
        (user_repo, role_repo, use_case)
    }

    #[tokio::test]
    async fn lists_users_of_role() {
        let tenant = make_tenant();
        let (user_repo, role_repo, use_case) = setup(&tenant);

        let role = Role::new(tenant.id(), "Admin".into(), None);
        let role_id = role.id();
        role_repo.memory.data.lock().unwrap().insert(role_id, role);

        for _ in 0..3 {
            let user = generate_user(tenant.id());
            let user_id = user.id();
            user_repo.memory.data.lock().unwrap().insert(user_id, user);
            role_repo
                .assignments
                .lock()
                .unwrap()
                .insert((role_id, user_id));
        }
        // Noise: a user not in the role.
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
                role_id,
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(response.total, 3);
        assert_eq!(response.items.len(), 3);
    }

    #[tokio::test]
    async fn fails_when_role_not_found() {
        let tenant = make_tenant();
        let (_, _, use_case) = setup(&tenant);

        let err = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                role_id: Uuid::new_v4(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
