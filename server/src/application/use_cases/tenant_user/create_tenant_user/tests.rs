#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant_user::create_tenant_user::{
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
        utils::password_hasher::compare_password,
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

    fn make_command(tenant_id: Uuid) -> Command {
        Command {
            tenant_id,
            email: "user@example.com".to_string(),
            display_name: Some("Display Name".to_string()),
            first_name: "First".to_string(),
            last_name: "Last".to_string(),
            is_email_verified: false,
            temporary_password: "raw_password".to_string(),
        }
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

        let use_case = UseCase::new(
            uow_factory,
            crate::testing::password_hasher::test_password_hasher(),
        );

        (user_repo, use_case, outbox)
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn creates_user_successfully() {
        let tenant = make_tenant();
        let (user_repo, use_case) = setup(&tenant);

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap();

        let saved = user_repo
            .find_by_id(response.user_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(response.email, "user@example.com");
        assert_eq!(response.first_name, "First");
        assert_eq!(response.last_name, "Last");
        assert_eq!(response.display_name, Some("Display Name".to_string()));
        assert_eq!(user_repo.memory.data.lock().unwrap().len(), 1);
        assert_eq!(saved.email(), "user@example.com");
    }

    #[tokio::test]
    async fn password_is_hashed_with_tenant_secret() {
        let tenant = make_tenant();
        let (user_repo, use_case) = setup(&tenant);

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap();

        let saved = user_repo
            .find_by_id(response.user_id)
            .await
            .unwrap()
            .unwrap();
        let saved_hash = saved.credentials().unwrap().password_hash();

        // Argon2 uses a random salt, so two hashes of the same password differ.
        // Verify by checking the stored hash validates against the raw password
        // (with the tenant secret), rather than comparing hash strings.
        assert!(saved_hash.starts_with("$argon2id$v"));
        assert!(compare_password(
            saved_hash,
            "raw_password",
            tenant.password_hash_secret()
        ));
    }

    #[tokio::test]
    async fn creates_user_without_display_name() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let mut command = make_command(tenant.id());
        command.display_name = None;

        let response = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap();

        assert_eq!(response.display_name, None);
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let command = make_command(Uuid::new_v4()); // unknown tenant

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_email_already_exists() {
        let tenant = make_tenant();
        let (user_repo, use_case) = setup(&tenant);

        // Seed an existing user with the same email directly into committed data.
        let existing = generate_user(tenant.id());
        user_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        let mut command = make_command(tenant.id());
        command.email = existing_email(&user_repo);

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Conflict(_)));
    }

    // Returns the email of the first user in the repository.
    fn existing_email(repo: &InMemoryUserRepository) -> String {
        repo.memory
            .data
            .lock()
            .unwrap()
            .values()
            .next()
            .unwrap()
            .email()
            .to_string()
    }

    // ── Outbound events ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn enqueues_a_user_created_event_with_the_admin_source() {
        let tenant = make_tenant();
        let (_, use_case, outbox) = setup_with_outbox(&tenant);

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap();

        let stored = outbox.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let event = stored.values().next().unwrap();
        assert_eq!(event.tenant_id(), tenant.id());
        assert_eq!(event.event_type().as_str(), "user.created");
        assert!(!event.is_published());

        let data = &event.payload()["data"];
        assert_eq!(data["user_id"], response.user_id.to_string());
        assert_eq!(data["email"], "user@example.com");
        assert_eq!(data["source"], "admin");
        assert_eq!(data["email_verified"], false);
    }

    // THE property the whole outbox design exists for: the event and the user share a
    // transaction, so a use case that fails partway leaves NEITHER behind. Without
    // this, a downstream could be told about a user that does not exist.
    #[tokio::test]
    async fn a_failed_creation_leaves_no_event_behind() {
        let tenant = make_tenant();
        let (user_repo, use_case, outbox) = setup_with_outbox(&tenant);

        // First call succeeds and commits.
        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap();
        assert_eq!(outbox.memory.data.lock().unwrap().len(), 1);

        // Second call fails on the duplicate-email guard, before commit.
        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Conflict(_)));

        // Neither a second user nor a second event was committed.
        assert_eq!(user_repo.memory.data.lock().unwrap().len(), 1);
        assert_eq!(outbox.memory.data.lock().unwrap().len(), 1);
    }
}
