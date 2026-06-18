#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::role::list_user_roles::{query::Query, use_case::UseCase},
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
            false,
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

        let use_case = UseCase::new(role_repo.clone(), user_repo.clone(), tenant_repo);
        (user_repo, role_repo, use_case)
    }

    #[tokio::test]
    async fn lists_roles_for_user() {
        let tenant = make_tenant();
        let (user_repo, role_repo, use_case) = setup(&tenant);

        let user = generate_user(tenant.id());
        let user_id = user.id();
        user_repo.memory.data.lock().unwrap().insert(user_id, user);

        let r1 = Role::new(tenant.id(), "Admin".into(), None);
        let r2 = Role::new(tenant.id(), "Editor".into(), None);
        let r3 = Role::new(tenant.id(), "Viewer".into(), None); // user not assigned
        let (r1_id, r2_id) = (r1.id(), r2.id());
        {
            let mut data = role_repo.memory.data.lock().unwrap();
            data.insert(r1.id(), r1);
            data.insert(r2.id(), r2);
            data.insert(r3.id(), r3);
        }
        role_repo
            .assignments
            .lock()
            .unwrap()
            .insert((r1_id, user_id));
        role_repo
            .assignments
            .lock()
            .unwrap()
            .insert((r2_id, user_id));

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                user_id,
            })
            .await
            .unwrap();

        assert_eq!(response.len(), 2);
        assert!(
            response
                .iter()
                .all(|r| r.role_id == r1_id || r.role_id == r2_id)
        );
    }

    #[tokio::test]
    async fn returns_empty_when_user_has_no_roles() {
        let tenant = make_tenant();
        let (user_repo, _role_repo, use_case) = setup(&tenant);

        let user = generate_user(tenant.id());
        let user_id = user.id();
        user_repo.memory.data.lock().unwrap().insert(user_id, user);

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                user_id,
            })
            .await
            .unwrap();

        assert_eq!(response.len(), 0);
    }

    #[tokio::test]
    async fn fails_when_user_not_found() {
        let tenant = make_tenant();
        let (_, _, use_case) = setup(&tenant);

        let err = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                user_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::UserNotFound(_)));
    }
}
