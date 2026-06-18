#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::{
            ports::email_sender::EmailSender,
            use_cases::authentication::register::{command::Command, use_case::UseCase},
        },
        domain::{
            entities::{
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_email_verification_token_repository::InMemoryEmailVerificationTokenRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            recording_email_sender::RecordingEmailSender,
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
            MfaPolicy::default(),
        )
    }

    fn setup(tenant_id: Uuid, can_self_register: bool) -> Ctx {
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        let client = make_client(tenant_id);
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
            false,
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

        let factory = InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_oauth_client_repository(clients)
            .with_email_verification_token_repository(tokens.clone());

        let emails = Arc::new(RecordingEmailSender::new());

        let use_case = UseCase::new(Arc::new(factory), emails.clone() as Arc<dyn EmailSender>);

        Ctx {
            use_case,
            users,
            tokens,
            emails,
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
        let (to, code) = &sent[0];
        assert_eq!(to, "new.user@example.com");
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
        // The emailed code hashes to the stored token_hash.
        assert_eq!(
            crate::utils::oauth_crypto::sha256_hex(code),
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
}
