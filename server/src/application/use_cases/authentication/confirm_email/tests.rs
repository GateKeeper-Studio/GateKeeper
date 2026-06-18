#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::use_cases::authentication::confirm_email::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                email_verification_token::EmailVerificationToken,
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
                user::User,
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
                in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
                in_memory_email_verification_token_repository::InMemoryEmailVerificationTokenRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
        utils::oauth_crypto::sha256_hex,
    };

    const CLIENT_ID: &str = "01890000-0000-7000-8000-000000000abc";
    const REDIRECT_URI: &str = "https://app.example.com/callback";
    const RAW_CODE: &str = "123456";

    struct Ctx {
        use_case: UseCase,
        users: Arc<InMemoryUserRepository>,
        tokens: Arc<InMemoryEmailVerificationTokenRepository>,
        codes: Arc<InMemoryAuthorizationCodeRepository>,
        consents: Arc<InMemoryConsentGrantRepository>,
        user_id: Uuid,
        email: String,
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

    fn setup(tenant_id: Uuid, consumed: bool, expired: bool) -> Ctx {
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
            "tenant_secret".to_string(),
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

        let users = Arc::new(InMemoryUserRepository::new());
        let user = User::new(tenant_id, "new.user@example.com".to_string(), None, None);
        let user_id = user.id();
        let email = user.email().to_string();
        users.memory.data.lock().unwrap().insert(user.id(), user);

        let tokens = Arc::new(InMemoryEmailVerificationTokenRepository::new());
        let expires_at = if expired {
            Utc::now() - Duration::hours(1)
        } else {
            Utc::now() + Duration::minutes(15)
        };
        let mut token =
            EmailVerificationToken::new(tenant_id, user_id, sha256_hex(RAW_CODE), expires_at);
        if consumed {
            token.consume(Utc::now());
        }
        tokens.memory.data.lock().unwrap().insert(token.id(), token);

        let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
        let consents = Arc::new(InMemoryConsentGrantRepository::new());

        let factory = InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_oauth_client_repository(clients)
            .with_email_verification_token_repository(tokens.clone())
            .with_authorization_code_repository(codes.clone())
            .with_consent_grant_repository(consents.clone());

        Ctx {
            use_case: UseCase::new(Arc::new(factory)),
            users,
            tokens,
            codes,
            consents,
            user_id,
            email,
        }
    }

    fn make_command(email: &str) -> Command {
        Command {
            client_id: CLIENT_ID.to_string(),
            token: RAW_CODE.to_string(),
            email: email.to_string(),
            redirect_uri: REDIRECT_URI.to_string(),
            scope: "openid email".to_string(),
            state: "xyz".to_string(),
            code_challenge: "challenge".to_string(),
            code_challenge_method: "S256".to_string(),
            response_type: "code".to_string(),
            nonce: Some("n-1".to_string()),
        }
    }

    #[tokio::test]
    async fn verifies_email_consumes_token_and_mints_code() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, false, false);

        let response = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap();

        assert_eq!(response.state, "xyz");
        assert_eq!(response.redirect_uri, REDIRECT_URI);
        assert!(!response.authorization_code.is_empty());

        let user = ctx
            .users
            .memory
            .data
            .lock()
            .unwrap()
            .get(&ctx.user_id)
            .cloned()
            .unwrap();
        assert!(user.is_email_verified());

        let token = ctx
            .tokens
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .next()
            .cloned()
            .unwrap();
        assert!(token.is_consumed());

        let codes = ctx.codes.memory.data.lock().unwrap();
        assert_eq!(codes.len(), 1);
        let code = codes.values().next().unwrap();
        assert_eq!(code.code_hash(), sha256_hex(&response.authorization_code));
        let ttl = code.expires_at() - code.created_at();
        assert!(ttl.num_seconds() > 0 && ttl.num_seconds() <= 60);

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
    async fn fails_on_wrong_code_and_increments_attempts() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, false, false);

        let mut command = make_command(&ctx.email);
        command.token = "000000".to_string();

        let err = ctx.use_case.execute(command).await.unwrap_err();
        assert!(matches!(err, DomainError::EmailVerificationError(_)));

        // Attempt counter was persisted.
        let token = ctx
            .tokens
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .next()
            .cloned()
            .unwrap();
        assert_eq!(token.attempts(), 1);
        assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn fails_after_max_attempts_exhausted() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, false, false);

        // Exhaust all allowed attempts with wrong codes.
        for _ in 0..EmailVerificationToken::MAX_ATTEMPTS {
            let mut command = make_command(&ctx.email);
            command.token = "000000".to_string();
            let _ = ctx.use_case.execute(command).await;
        }

        // Correct code is now rejected too.
        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::EmailVerificationError(_)));
        assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn fails_on_consumed_token() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true, false);

        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::EmailVerificationError(_)));
    }

    #[tokio::test]
    async fn fails_on_expired_token() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, false, true);

        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::EmailVerificationError(_)));
    }

    #[tokio::test]
    async fn fails_on_redirect_uri_mismatch() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, false, false);

        let mut command = make_command(&ctx.email);
        command.redirect_uri = "https://evil.example.com/callback".to_string();

        let err = ctx.use_case.execute(command).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }
}
