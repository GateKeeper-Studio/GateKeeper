#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use uuid::Uuid;

    use serde_json::json;

    fn en(value: &str) -> HashMap<String, String> {
        HashMap::from([("en".to_string(), value.to_string())])
    }

    use crate::{
        application::use_cases::authentication::register::{command::Command, use_case::UseCase},
        domain::{
            entities::{
                custom_field_definition::{CustomFieldDefinition, CustomFieldType},
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::{
            fake_email_renderer::var_from_text,
            in_memory_repositories::{
                in_memory_custom_field_definition_repository::InMemoryCustomFieldDefinitionRepository,
                in_memory_email_verification_token_repository::InMemoryEmailVerificationTokenRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_outbox_event_repository::InMemoryOutboxEventRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            recording_email_sender::RecordingEmailSender,
            templated_email_support::recording_templated_email_sender,
        },
        utils::password_hasher::compare_password,
    };

    const CLIENT_ID: &str = "01890000-0000-7000-8000-000000000abc";
    const REDIRECT_URI: &str = "https://app.example.com/callback";
    const PASSWORD_SECRET: &str = "tenant_secret";

    struct Ctx {
        use_case: UseCase,
        users: Arc<InMemoryUserRepository>,
        tokens: Arc<InMemoryEmailVerificationTokenRepository>,
        emails: Arc<RecordingEmailSender>,
        custom_fields: Arc<InMemoryCustomFieldDefinitionRepository>,
        outbox: Arc<InMemoryOutboxEventRepository>,
    }

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::new(
            tenant_id,
            Uuid::parse_str(CLIENT_ID).unwrap(),
            None,
            ClientType::Public,
            "Test App".to_string(),
            vec![REDIRECT_URI.to_string()],
            vec![],
            vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            vec!["authorization_code".to_string()],
            true,
            true,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
    }

    fn setup(tenant_id: Uuid, can_self_register: bool) -> Ctx {
        setup_with_client(tenant_id, can_self_register, make_client(tenant_id))
    }

    fn setup_with_client(tenant_id: Uuid, can_self_register: bool, client: OAuthClient) -> Ctx {
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client);

        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "Test Tenant".to_string(),
            None,
            PASSWORD_SECRET.to_string(),
            true,
            can_self_register,
            true,
            uuid::Uuid::now_v7(),
            chrono::Utc::now(),
            None,
        );
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant);

        let users = Arc::new(InMemoryUserRepository::new());
        let tokens = Arc::new(InMemoryEmailVerificationTokenRepository::new());
        let custom_fields = Arc::new(InMemoryCustomFieldDefinitionRepository::new());
        let outbox = Arc::new(InMemoryOutboxEventRepository::new());

        let factory = InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_oauth_client_repository(clients)
            .with_email_verification_token_repository(tokens.clone())
            .with_custom_field_definition_repository(custom_fields.clone())
            .with_outbox_event_repository(outbox.clone());

        let (templated_email, emails) = recording_templated_email_sender();

        let use_case = UseCase::new(Arc::new(factory), templated_email);

        Ctx {
            use_case,
            users,
            tokens,
            emails,
            custom_fields,
            outbox,
        }
    }

    fn make_command() -> Command {
        Command {
            client_id: CLIENT_ID.to_string(),
            email: "new.user@example.com".to_string(),
            password: "Sup3rSecret!".to_string(),
            display_name: Some("New User".to_string()),
            first_name: "New".to_string(),
            last_name: "User".to_string(),
            custom_field_values: None,
        }
    }

    #[tokio::test]
    async fn registers_unverified_user_and_emails_code() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let response = ctx.use_case.execute(make_command()).await.unwrap();
        assert!(!response.message.is_empty());

        // The user exists and is unverified; password is hashed with the tenant secret.
        let users = ctx.users.memory.data.lock().unwrap();
        assert_eq!(users.len(), 1);
        let saved = users.values().next().unwrap();
        assert_eq!(saved.email(), "new.user@example.com");
        assert!(!saved.is_email_verified());
        let hash = saved.credentials().unwrap().password_hash();
        assert!(compare_password(hash, "Sup3rSecret!", PASSWORD_SECRET));

        // Exactly one verification token was persisted (as a hash, not raw).
        let tokens = ctx.tokens.memory.data.lock().unwrap();
        assert_eq!(tokens.len(), 1);
        let token = tokens.values().next().unwrap();
        assert_eq!(token.user_id(), saved.id());
        assert!(!token.is_consumed());
        assert_eq!(token.attempts(), 0);

        // The email carries a 6-digit numeric code.
        let sent = ctx.emails.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "new.user@example.com");
        let code = var_from_text(&sent[0].text, "code").unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
        // The emailed code hashes to the stored token_hash.
        assert_eq!(
            crate::utils::oauth_crypto::sha256_hex(&code),
            token.token_hash()
        );
    }

    #[tokio::test]
    async fn fails_when_self_registration_disabled() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, false);

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)));
        assert_eq!(ctx.users.memory.data.lock().unwrap().len(), 0);
    }

    // Users of an external-login-only client never pick a password — they are
    // provisioned by the social callback instead. Refused even though the tenant
    // itself allows self-registration.
    #[tokio::test]
    async fn fails_when_client_is_external_login_only() {
        let tenant_id = Uuid::now_v7();
        let mut client = make_client(tenant_id);
        client.set_external_login_only(true);
        let ctx = setup_with_client(tenant_id, true, client);

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)));
        assert_eq!(ctx.users.memory.data.lock().unwrap().len(), 0);
        assert_eq!(ctx.emails.sent.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn fails_when_client_unknown() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let mut command = make_command();
        command.client_id = "does_not_exist".to_string();

        let err = ctx.use_case.execute(command).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidClient(_)));
    }

    #[tokio::test]
    async fn fails_when_email_already_exists() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        ctx.use_case.execute(make_command()).await.unwrap();

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::UserAlreadyExists(_)));
    }

    #[tokio::test]
    async fn normalizes_email_and_rejects_case_variant_duplicate() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        // Register with a mixed-case address.
        let mut first = make_command();
        first.email = "Mixed.Case@Example.com".to_string();
        ctx.use_case.execute(first).await.unwrap();

        // It is stored canonically (lowercased).
        {
            let users = ctx.users.memory.data.lock().unwrap();
            assert_eq!(
                users.values().next().unwrap().email(),
                "mixed.case@example.com"
            );
        }

        // A different-case variant of the same address is rejected as a duplicate.
        let mut second = make_command();
        second.email = "MIXED.CASE@example.COM".to_string();
        let err = ctx.use_case.execute(second).await.unwrap_err();
        assert!(matches!(err, DomainError::UserAlreadyExists(_)));
        assert_eq!(ctx.users.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn collects_and_validates_custom_field_values_at_signup() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let field = CustomFieldDefinition::new(
            tenant_id,
            "referral_source".to_string(),
            en("How did you hear about us?"),
            None,
            CustomFieldType::Text,
            None,
            true,
            0,
        );
        ctx.custom_fields
            .memory
            .data
            .lock()
            .unwrap()
            .insert(field.id(), field);

        let mut command = make_command();
        command.custom_field_values = Some(json!({"referral_source": "a friend"}));
        ctx.use_case.execute(command).await.unwrap();

        let users = ctx.users.memory.data.lock().unwrap();
        let saved = users.values().next().unwrap();
        let values = saved.profile().unwrap().custom_field_values().unwrap();
        assert_eq!(values["referral_source"], "a friend");
    }

    #[tokio::test]
    async fn rejects_signup_missing_required_custom_field() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let field = CustomFieldDefinition::new(
            tenant_id,
            "referral_source".to_string(),
            en("How did you hear about us?"),
            None,
            CustomFieldType::Text,
            None,
            true,
            0,
        );
        ctx.custom_fields
            .memory
            .data
            .lock()
            .unwrap()
            .insert(field.id(), field);

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidCustomFieldValue(_)));
        assert_eq!(ctx.users.memory.data.lock().unwrap().len(), 0);
    }

    // ── Outbound events ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn enqueues_a_user_created_event_with_the_self_registration_source() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        ctx.use_case.execute(make_command()).await.unwrap();

        let stored = ctx.outbox.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let event = stored.values().next().unwrap();
        assert_eq!(event.tenant_id(), tenant_id);
        assert_eq!(event.event_type().as_str(), "user.created");

        let data = &event.payload()["data"];
        assert_eq!(data["email"], "new.user@example.com");
        assert_eq!(data["source"], "self_registration");
        // Registration does not verify the email, and the event must say so.
        assert_eq!(data["email_verified"], false);
    }

    // The event shares the user's transaction, so a rejected registration announces
    // nothing.
    #[tokio::test]
    async fn a_rejected_registration_leaves_no_event_behind() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, false); // self-registration disabled

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)));

        assert_eq!(ctx.users.memory.data.lock().unwrap().len(), 0);
        assert_eq!(ctx.outbox.memory.data.lock().unwrap().len(), 0);
    }
}
