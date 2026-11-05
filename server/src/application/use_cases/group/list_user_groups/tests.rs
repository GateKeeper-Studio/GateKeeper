#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::group::list_user_groups::{query::Query, use_case::UseCase},
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

        let use_case = UseCase::new(group_repo.clone(), user_repo.clone(), tenant_repo);
        (user_repo, group_repo, use_case)
    }

    #[tokio::test]
    async fn lists_groups_for_user() {
        let tenant = make_tenant();
        let (user_repo, group_repo, use_case) = setup(&tenant);

        let user = generate_user(tenant.id());
        let user_id = user.id();
        user_repo.memory.data.lock().unwrap().insert(user_id, user);

        let g1 = Group::new(tenant.id(), "Eng".into(), None);
        let g2 = Group::new(tenant.id(), "Design".into(), None);
        let g3 = Group::new(tenant.id(), "Ops".into(), None); // user not a member
        let (g1_id, g2_id) = (g1.id(), g2.id());
        {
            let mut data = group_repo.memory.data.lock().unwrap();
            data.insert(g1.id(), g1);
            data.insert(g2.id(), g2);
            data.insert(g3.id(), g3);
        }
        group_repo.members.lock().unwrap().insert((g1_id, user_id));
        group_repo.members.lock().unwrap().insert((g2_id, user_id));

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
                .all(|g| g.group_id == g1_id || g.group_id == g2_id)
        );
    }

    #[tokio::test]
    async fn returns_empty_when_user_has_no_groups() {
        let tenant = make_tenant();
        let (user_repo, _group_repo, use_case) = setup(&tenant);

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

        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
