#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant_user::list_tenant_users::{query::Query, use_case::UseCase},
        domain::{
            entities::{
                external_identity_provider::ExternalProviderKind, social_identity::SocialIdentity,
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_social_identity_repository::InMemorySocialIdentityRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            user_factory::{generate_user, generate_users},
        },
    };

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn make_tenant() -> Tenant {
        Tenant::new(
            "Test Tenant".to_string(), // name
            None,                      // description
            "test_secret".to_string(), // password_hash_secret
            true,                      // is_active
            true,                      // can_self_register
            true,                      // can_self_forgot_password
            uuid::Uuid::now_v7(),      // organization_id
        )
    }

    fn setup(
        tenant: &Tenant,
    ) -> (
        Arc<InMemoryUserRepository>,
        Arc<InMemorySocialIdentityRepository>,
        UseCase,
    ) {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let social_repo = Arc::new(InMemorySocialIdentityRepository::new());

        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let use_case = UseCase::new(user_repo.clone(), tenant_repo.clone(), social_repo.clone());

        (user_repo, social_repo, use_case)
    }

    fn seed_users(repo: &InMemoryUserRepository, tenant_id: Uuid, quantity: usize) {
        let users = generate_users(tenant_id, quantity);
        let mut data = repo.memory.data.lock().unwrap();
        for user in users {
            data.insert(user.id(), user);
        }
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn returns_first_page_with_defaults_when_no_pagination_provided() {
        let tenant = make_tenant();
        let (user_repo, _social_repo, use_case) = setup(&tenant);
        seed_users(&user_repo, tenant.id(), 25);

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
    async fn returns_correct_items_for_requested_page() {
        let tenant = make_tenant();
        let (user_repo, _social_repo, use_case) = setup(&tenant);
        seed_users(&user_repo, tenant.id(), 12);

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                page: Some(2),
                page_size: Some(5),
            })
            .await
            .unwrap();

        assert_eq!(response.page, 2);
        assert_eq!(response.page_size, 5);
        assert_eq!(response.total, 12);
        assert_eq!(response.items.len(), 5);
    }

    #[tokio::test]
    async fn last_page_returns_remaining_items() {
        let tenant = make_tenant();
        let (user_repo, _social_repo, use_case) = setup(&tenant);
        seed_users(&user_repo, tenant.id(), 12);

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                page: Some(3),
                page_size: Some(5),
            })
            .await
            .unwrap();

        // 12 users, page_size 5: pages 1→5, 2→5, 3→2
        assert_eq!(response.items.len(), 2);
        assert_eq!(response.total, 12);
    }

    #[tokio::test]
    async fn returns_empty_list_when_tenant_has_no_users() {
        let tenant = make_tenant();
        let (_, _social_repo, use_case) = setup(&tenant);

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(response.items.len(), 0);
        assert_eq!(response.total, 0);
    }

    #[tokio::test]
    async fn does_not_return_users_from_other_tenants() {
        let tenant = make_tenant();
        let other_tenant = make_tenant();
        let (user_repo, _social_repo, use_case) = setup(&tenant);

        seed_users(&user_repo, tenant.id(), 3);
        seed_users(&user_repo, other_tenant.id(), 10); // noise

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(response.total, 3);
        assert!(response.items.iter().all(|u| {
            user_repo
                .memory
                .data
                .lock()
                .unwrap()
                .get(&u.user_id)
                .map(|u| u.tenant_id() == tenant.id())
                .unwrap_or(false)
        }));
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let tenant = make_tenant();
        let (_, _social_repo, use_case) = setup(&tenant);

        let err = use_case
            .execute(Query {
                tenant_id: Uuid::new_v4(), // unknown tenant
                page: None,
                page_size: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn reports_login_methods_password_and_social_providers() {
        let tenant = make_tenant();
        let (user_repo, social_repo, use_case) = setup(&tenant);

        // generate_user attaches password credentials → has_password is true.
        let user = generate_user(tenant.id());
        let user_id = user.id();
        user_repo.memory.data.lock().unwrap().insert(user_id, user);

        // Link a GitHub identity to the same user.
        let identity = SocialIdentity::new(
            user_id,
            tenant.id(),
            ExternalProviderKind::Github,
            "gh-subject-1".to_string(),
            Some("user@example.com".to_string()),
        );
        social_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(identity.id(), identity);

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        let item = response
            .items
            .iter()
            .find(|u| u.user_id == user_id)
            .expect("seeded user should be listed");
        assert!(item.has_password);
        assert_eq!(item.providers, vec!["github".to_string()]);
    }
}
