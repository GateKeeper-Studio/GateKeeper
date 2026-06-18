#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use chrono::Utc;

    use crate::{
        application::use_cases::oauth::authorize_complete::{command::Command, use_case::UseCase},
        domain::{
            entities::{
                consent_grant::ConsentGrant,
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
                in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            user_factory::{TEST_PASSWORD, TEST_PASSWORD_SECRET, generate_user},
        },
        utils::oauth_crypto::sha256_hex,
    };

    struct Ctx {
        use_case: UseCase,
        client: OAuthClient,
        codes: Arc<InMemoryAuthorizationCodeRepository>,
        consents: Arc<InMemoryConsentGrantRepository>,
        email: String,
        user_id: Uuid,
    }

    fn client_id() -> Uuid {
        Uuid::parse_str("01890000-0000-7000-8000-000000000abc").unwrap()
    }

    fn make_client(tenant_id: Uuid, require_consent: bool) -> OAuthClient {
        OAuthClient::new(
            tenant_id,
            client_id(),
            None,
            ClientType::Public,
            "Test App".to_string(),
            vec!["https://app.example.com/callback".to_string()],
            vec![],
            vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            vec!["authorization_code".to_string()],
            true,
            require_consent,
            MfaPolicy::default(),
        )
    }

    fn setup(tenant_id: Uuid, require_consent: bool) -> Ctx {
        let client = make_client(tenant_id, require_consent);

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
        let consents = Arc::new(InMemoryConsentGrantRepository::new());

        // A tenant whose pepper matches the one the user factory hashes with, so the
        // use case's password verification succeeds.
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "Test Tenant".to_string(),
            None,
            TEST_PASSWORD_SECRET.to_string(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
            false,
            Utc::now(),
            None,
        );
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant);

        // A real user with credentials (hashed with TEST_PASSWORD_SECRET).
        let users = Arc::new(InMemoryUserRepository::new());
        let user = generate_user(tenant_id);
        let email = user.email().to_string();
        let user_id = user.id();
        users.memory.data.lock().unwrap().insert(user.id(), user);

        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_oauth_client_repository(clients)
            .with_authorization_code_repository(codes.clone())
            .with_consent_grant_repository(consents.clone());

        Ctx {
            use_case: UseCase::new(Arc::new(factory)),
            client,
            codes,
            consents,
            email,
            user_id,
        }
    }

    fn make_command(tenant_id: Uuid, email: &str, approved: bool) -> Command {
        Command {
            tenant_id,
            client_id: client_id().to_string(),
            email: email.to_string(),
            password: TEST_PASSWORD.to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            scopes: vec!["openid".to_string(), "email".to_string()],
            state: "xyz".to_string(),
            code_challenge: "challenge".to_string(),
            code_challenge_method: "S256".to_string(),
            nonce: Some("n-1".to_string()),
            user_approved: approved,
        }
    }

    #[tokio::test]
    async fn issues_code_and_persists_consent_when_approved() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let response = ctx
            .use_case
            .execute(make_command(tenant_id, &ctx.email, true))
            .await
            .unwrap();

        assert_eq!(response.state, "xyz");
        assert!(!response.code.is_empty());

        // The code is persisted as a hash, never raw, with a ~60s expiry.
        let stored = ctx.codes.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let code = stored.values().next().unwrap();
        assert_eq!(code.code_hash(), sha256_hex(&response.code));
        assert_ne!(code.code_hash(), response.code);
        let ttl = code.expires_at() - code.created_at();
        assert!(ttl.num_seconds() > 0 && ttl.num_seconds() <= 60);

        // A consent grant now covers the requested scopes.
        let grants = ctx.consents.memory.data.lock().unwrap();
        assert_eq!(grants.len(), 1);
        assert!(
            grants
                .values()
                .next()
                .unwrap()
                .covers(&["openid".to_string(), "email".to_string()])
        );
    }

    #[tokio::test]
    async fn requires_consent_when_not_yet_granted_and_not_approved() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let err = ctx
            .use_case
            .execute(make_command(tenant_id, &ctx.email, false))
            .await
            .unwrap_err();

        match err {
            DomainError::ConsentRequired(missing) => assert_eq!(missing, "openid email"),
            other => panic!("expected ConsentRequired, got {other:?}"),
        }
        // Nothing persisted on the consent-required short-circuit.
        assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn skips_consent_when_existing_grant_covers_scopes() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        // Pre-seed a covering grant for (user, client).
        let grant = ConsentGrant::new(
            tenant_id,
            ctx.user_id,
            ctx.client.id(),
            vec!["openid".to_string(), "email".to_string()],
        );
        ctx.consents
            .memory
            .data
            .lock()
            .unwrap()
            .insert(grant.id(), grant);

        // user_approved = false, yet it succeeds because consent is already covered.
        let response = ctx
            .use_case
            .execute(make_command(tenant_id, &ctx.email, false))
            .await
            .unwrap();
        assert!(!response.code.is_empty());
        assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn auto_approves_when_client_does_not_require_consent() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, false); // require_consent = false

        let response = ctx
            .use_case
            .execute(make_command(tenant_id, &ctx.email, false))
            .await
            .unwrap();
        assert!(!response.code.is_empty());
    }

    #[tokio::test]
    async fn rejects_wrong_password() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let mut command = make_command(tenant_id, &ctx.email, true);
        command.password = "not-the-password".to_string();

        let err = ctx.use_case.execute(command).await.unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
        // No code is issued for a failed authentication.
        assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn rejects_unknown_user() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let err = ctx
            .use_case
            .execute(make_command(tenant_id, "nobody@example.com", true))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }
}
