#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::{
            ports::email_sender::EmailSender,
            services::templated_email_sender::TemplatedEmailSender,
            shared::token_policy::NEW_ACCOUNT_PASSWORD_TTL_HOURS,
            use_cases::tenant_user::create_tenant_user::{command::Command, use_case::UseCase},
        },
        domain::{
            entities::{
                email_provider::{EmailProvider, SmtpEncryption},
                tenant::Tenant,
            },
            errors::DomainError,
            repositories::user_repository::UserRepository,
        },
        testing::{
            fake_email_renderer::{FakeEmailRenderer, var_from_text},
            in_memory_repositories::{
                in_memory_email_provider_repository::InMemoryEmailProviderRepository,
                in_memory_email_template_repository::InMemoryEmailTemplateRepository,
                in_memory_outbox_event_repository::InMemoryOutboxEventRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            recording_email_sender::RecordingEmailSender,
            templated_email_support::recording_templated_email_sender,
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
            temporary_password: None,
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
        let (user_repo, use_case, outbox, _) = setup_full(tenant, true, None);
        (user_repo, use_case, outbox)
    }

    // A tenant with (or without) an active default provider; the transport records
    // what it sends unless `transport` replaces it.
    fn setup_full(
        tenant: &Tenant,
        with_provider: bool,
        transport: Option<Arc<dyn EmailSender>>,
    ) -> (
        Arc<InMemoryUserRepository>,
        UseCase,
        Arc<InMemoryOutboxEventRepository>,
        Arc<RecordingEmailSender>,
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

        let providers = Arc::new(InMemoryEmailProviderRepository::new());
        if with_provider {
            let provider = EmailProvider::new(
                tenant.id(),
                "Main".to_string(),
                "smtp.example.com".to_string(),
                587,
                "mailer".to_string(),
                "v1:sealed".to_string(),
                "no-reply@example.com".to_string(),
                None,
                SmtpEncryption::StartTls,
                true,
            );
            providers
                .memory
                .data
                .lock()
                .unwrap()
                .insert(provider.id(), provider);
        }

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(user_repo.clone(), tenant_repo.clone())
                .with_outbox_event_repository(outbox.clone())
                .with_email_provider_repository(providers),
        );

        let (recording_templated, recording) = recording_templated_email_sender();
        let templated = match transport {
            Some(transport) => Arc::new(TemplatedEmailSender::new(
                Arc::new(InMemoryEmailTemplateRepository::new()),
                Arc::new(FakeEmailRenderer::new()),
                transport,
            )),
            None => recording_templated,
        };

        let use_case = UseCase::new(
            uow_factory,
            crate::testing::password_hasher::test_password_hasher(),
            templated,
            true,
        );

        (user_repo, use_case, outbox, recording)
    }

    struct FailingEmailSender;

    #[async_trait]
    impl EmailSender for FailingEmailSender {
        async fn send(
            &self,
            _provider: Option<&EmailProvider>,
            _to: &str,
            _subject: &str,
            _html_body: &str,
            _text_body: &str,
        ) -> Result<(), DomainError> {
            Err(DomainError::Upstream(
                "the email could not be delivered".to_string(),
            ))
        }

        async fn send_test_email(
            &self,
            _provider: Option<&EmailProvider>,
            _to: &str,
        ) -> Result<(), DomainError> {
            Ok(())
        }
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

    // The server makes the first password and only the user gets it, by email. It is
    // stored hashed with the tenant secret, single use and expiring.
    #[tokio::test]
    async fn the_first_password_is_emailed_to_the_user_and_never_returned() {
        let tenant = make_tenant();
        let (user_repo, use_case, _, email) = setup_full(&tenant, true, None);

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap();

        let sent = email.sent.lock().unwrap().clone();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "user@example.com");
        assert_eq!(sent[0].subject, "Your new Test Tenant account");
        let password = var_from_text(&sent[0].text, "password").expect("password in the email");
        assert!(response.temporary_password_emailed);
        assert!(!format!("{response:?}").contains(&password));

        let saved = user_repo
            .find_by_id(response.user_id)
            .await
            .unwrap()
            .unwrap();
        let credentials = saved.credentials().unwrap();
        // Argon2 uses a random salt, so two hashes of the same password differ.
        // Verify by checking the stored hash validates against the raw password
        // (with the tenant secret), rather than comparing hash strings.
        assert!(credentials.password_hash().starts_with("$argon2id$v"));
        assert!(compare_password(
            credentials.password_hash(),
            &password,
            tenant.password_hash_secret()
        ));
        assert!(credentials.should_change_password());
        assert!(!credentials.proves_account_owner());
        let expires_at = credentials.temporary_password_expires_at().unwrap();
        assert_eq!(expires_at, response.temporary_password_expires_at);
        let ttl = expires_at - Utc::now();
        assert!(ttl <= Duration::hours(NEW_ACCOUNT_PASSWORD_TTL_HOURS));
        assert!(ttl > Duration::hours(NEW_ACCOUNT_PASSWORD_TTL_HOURS) - Duration::minutes(1));
    }

    #[tokio::test]
    async fn a_password_chosen_by_the_caller_is_refused() {
        let tenant = make_tenant();
        let (user_repo, use_case, _, email) = setup_full(&tenant, true, None);
        let mut command = make_command(tenant.id());
        command.temporary_password = Some("raw_password".to_string());

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Validation(_)));
        assert!(user_repo.memory.data.lock().unwrap().is_empty());
        assert!(email.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn nothing_is_created_when_the_email_cannot_go_out() {
        let tenant = make_tenant();
        let (user_repo, use_case, outbox, _) = setup_full(&tenant, false, None);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Validation(_)));
        assert!(user_repo.memory.data.lock().unwrap().is_empty());
        assert!(outbox.memory.data.lock().unwrap().is_empty());
    }

    // The account exists once committed: a failed delivery is reported, not an error,
    // so the admin knows to send a new password with a reset.
    #[tokio::test]
    async fn a_failed_delivery_is_reported_in_the_answer() {
        let tenant = make_tenant();
        let (user_repo, use_case, _, _) =
            setup_full(&tenant, true, Some(Arc::new(FailingEmailSender)));

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant.id()),
            )
            .await
            .unwrap();

        assert!(!response.temporary_password_emailed);
        assert_eq!(user_repo.memory.data.lock().unwrap().len(), 1);
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
