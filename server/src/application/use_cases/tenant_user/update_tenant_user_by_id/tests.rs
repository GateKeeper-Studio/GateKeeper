#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant_user::update_tenant_user_by_id::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::tenant::Tenant, errors::DomainError,
            repositories::user_repository::UserRepository,
        },
        testing::{
            in_memory_repositories::{
                in_memory_outbox_event_repository::InMemoryOutboxEventRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            user_factory::generate_user,
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

    fn setup(tenant: &Tenant) -> (Arc<InMemoryUserRepository>, UseCase) {
        let (user_repo, use_case, _) = setup_with_outbox(tenant);
        (user_repo, use_case)
    }

    fn setup_with_outbox(
        tenant: &Tenant,
    ) -> (
        Arc<InMemoryUserRepository>,
        UseCase,
        Arc<InMemoryOutboxEventRepository>,
    ) {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let outbox = Arc::new(InMemoryOutboxEventRepository::new());

        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(user_repo.clone(), tenant_repo.clone())
                .with_outbox_event_repository(outbox.clone()),
        );

        let use_case = UseCase::new(uow_factory);

        (user_repo, use_case, outbox)
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn updates_profile_successfully() {
        let tenant = make_tenant();
        let (user_repo, use_case) = setup(&tenant);
        let user = generate_user(tenant.id());
        user_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());

        let response = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                user_id: user.id(),
                first_name: "NewFirst".to_string(),
                last_name: "NewLast".to_string(),
                display_name: Some("New Display".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.user_id, user.id());
        assert_eq!(response.first_name, "NewFirst");
        assert_eq!(response.last_name, "NewLast");
        assert_eq!(response.display_name, Some("New Display".to_string()));
        assert_eq!(response.email, user.email());
    }

    #[tokio::test]
    async fn persists_updated_profile_to_repository() {
        let tenant = make_tenant();
        let (user_repo, use_case) = setup(&tenant);
        let user = generate_user(tenant.id());
        user_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());

        use_case
            .execute(Command {
                tenant_id: tenant.id(),
                user_id: user.id(),
                first_name: "Persisted".to_string(),
                last_name: "Name".to_string(),
                display_name: None,
            })
            .await
            .unwrap();

        let saved = user_repo.find_by_id(user.id()).await.unwrap().unwrap();
        assert_eq!(saved.profile().unwrap().first_name(), "Persisted");
        assert_eq!(saved.profile().unwrap().last_name(), "Name");
        assert_eq!(saved.profile().unwrap().display_name(), None);
    }

    #[tokio::test]
    async fn fails_when_user_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                user_id: Uuid::new_v4(),
                first_name: "First".to_string(),
                last_name: "Last".to_string(),
                display_name: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::UserNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(Command {
                tenant_id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                first_name: "First".to_string(),
                last_name: "Last".to_string(),
                display_name: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TenantNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_user_belongs_to_different_tenant() {
        let tenant = make_tenant();
        let other_tenant = make_tenant();
        let (user_repo, use_case) = setup(&tenant);

        let user = generate_user(other_tenant.id());
        user_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());

        let err = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                user_id: user.id(),
                first_name: "First".to_string(),
                last_name: "Last".to_string(),
                display_name: None,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::UserNotFound(_)));
    }

    // ── Outbound events ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn enqueues_a_user_updated_event_carrying_the_new_profile() {
        let tenant = make_tenant();
        let (user_repo, use_case, outbox) = setup_with_outbox(&tenant);
        let user = generate_user(tenant.id());
        let user_id = user.id();
        user_repo.memory.data.lock().unwrap().insert(user_id, user);

        use_case
            .execute(Command {
                tenant_id: tenant.id(),
                user_id,
                display_name: Some("Renamed".to_string()),
                first_name: "New".to_string(),
                last_name: "Name".to_string(),
            })
            .await
            .unwrap();

        let stored = outbox.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let event = stored.values().next().unwrap();
        assert_eq!(event.event_type().as_str(), "user.updated");

        // The payload is frozen at enqueue time and must reflect the NEW values —
        // a subscriber replaying this hours later still gets the right state.
        let data = &event.payload()["data"];
        assert_eq!(data["user_id"], user_id.to_string());
        assert_eq!(data["display_name"], "Renamed");
        assert_eq!(data["first_name"], "New");
        assert_eq!(data["last_name"], "Name");
        // user.updated carries no source — it is not a provisioning event.
        assert!(data["source"].is_null());
    }

    #[tokio::test]
    async fn a_failed_update_leaves_no_event_behind() {
        let tenant = make_tenant();
        let (_, use_case, outbox) = setup_with_outbox(&tenant);

        let err = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                user_id: Uuid::now_v7(),
                display_name: None,
                first_name: "New".to_string(),
                last_name: "Name".to_string(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::UserNotFound(_)));

        assert_eq!(outbox.memory.data.lock().unwrap().len(), 0);
    }
}
