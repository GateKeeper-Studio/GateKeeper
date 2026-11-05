#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::use_cases::authentication::reset_password::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                password_reset_token::PasswordResetToken,
                refresh_token::RefreshToken,
                tenant::Tenant,
                user::User,
                user_credentials::UserCredentials,
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_password_reset_token_repository::InMemoryPasswordResetTokenRepository,
                in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
        utils::{
            oauth_crypto::sha256_hex,
            password_hasher::{compare_password, hash_password},
        },
    };

    const CLIENT_ID: &str = "01890000-0000-7000-8000-000000000abc";
    const REDIRECT_URI: &str = "https://app.example.com/callback";
    const SECRET: &str = "tenant_secret";
    const OLD_PASSWORD: &str = "old-password-123";
    const NEW_PASSWORD: &str = "new-password-456";
    const RAW_TOKEN: &str = "raw-reset-token-value";

    struct Ctx {
        use_case: UseCase,
        users: Arc<InMemoryUserRepository>,
        tokens: Arc<InMemoryPasswordResetTokenRepository>,
        refresh: Arc<InMemoryRefreshTokenRepository>,
        user_id: Uuid,
        reset_id: Uuid,
        refresh_id: Uuid,
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
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
    }

    fn setup(consumed: bool, expired: bool) -> Ctx {
        setup_ext(consumed, expired, false)
    }

    fn setup_ext(consumed: bool, expired: bool, external_login_only: bool) -> Ctx {
        let tenant_id = Uuid::now_v7();
        let client_uuid = Uuid::parse_str(CLIENT_ID).unwrap();

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        let mut client = make_client(tenant_id);
        client.set_external_login_only(external_login_only);
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
            SECRET.to_string(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
            Utc::now(),
            None,
        );
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant);

        // Active user whose email is NOT yet verified (to prove the reset verifies it).
        let users = Arc::new(InMemoryUserRepository::new());
        let mut user = User::new(tenant_id, "user@example.com".to_string(), None, None);
        let old_hash = hash_password(OLD_PASSWORD, SECRET).unwrap();
        user.set_credentials(UserCredentials::new(
            user.id(),
            old_hash,
            "argon2".to_string(),
            false,
        ));
        let user_id = user.id();
        users.memory.data.lock().unwrap().insert(user.id(), user);

        let tokens = Arc::new(InMemoryPasswordResetTokenRepository::new());
        let expires_at = if expired {
            Utc::now() - Duration::hours(1)
        } else {
            Utc::now() + Duration::minutes(30)
        };
        let mut token =
            PasswordResetToken::new(tenant_id, user_id, sha256_hex(RAW_TOKEN), expires_at);
        if consumed {
            token.consume(Utc::now());
        }
        let reset_id = token.id();
        tokens.memory.data.lock().unwrap().insert(token.id(), token);

        // A live refresh token that must be revoked by a successful reset.
        let refresh = Arc::new(InMemoryRefreshTokenRepository::new());
        let rt = RefreshToken::new(
            tenant_id,
            sha256_hex("rt"),
            client_uuid,
            user_id,
            vec![],
            Utc::now() + Duration::days(30),
        );
        let refresh_id = rt.id();
        refresh.memory.data.lock().unwrap().insert(rt.id(), rt);

        let factory = InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_oauth_client_repository(clients)
            .with_password_reset_token_repository(tokens.clone())
            .with_refresh_token_repository(refresh.clone());

        Ctx {
            use_case: UseCase::new(
                Arc::new(factory),
                crate::testing::password_hasher::test_password_hasher(),
            ),
            users,
            tokens,
            refresh,
            user_id,
            reset_id,
            refresh_id,
        }
    }

    fn command(ctx: &Ctx, raw_token: &str) -> Command {
        Command {
            client_id: CLIENT_ID.to_string(),
            password_reset_token: raw_token.to_string(),
            password_reset_id: ctx.reset_id.to_string(),
            new_password: NEW_PASSWORD.to_string(),
        }
    }

    fn current_hash(ctx: &Ctx) -> String {
        ctx.users
            .memory
            .data
            .lock()
            .unwrap()
            .get(&ctx.user_id)
            .unwrap()
            .credentials()
            .unwrap()
            .password_hash()
            .to_string()
    }

    // An external-login-only client mints no reset tokens; a still-valid one issued
    // before the flag was turned on is refused rather than redeemed.
    #[tokio::test]
    async fn refuses_for_an_external_login_only_client() {
        let ctx = setup_ext(false, false, true);
        let before = current_hash(&ctx);

        let err = ctx
            .use_case
            .execute(command(&ctx, RAW_TOKEN))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::AccessDenied(_)));
        assert_eq!(current_hash(&ctx), before);
    }

    #[tokio::test]
    async fn resets_password_verifies_email_consumes_token_and_revokes_sessions() {
        let ctx = setup(false, false);

        ctx.use_case
            .execute(command(&ctx, RAW_TOKEN))
            .await
            .unwrap();

        // Password is now the new one (and no longer the old one).
        let hash = current_hash(&ctx);
        assert!(compare_password(&hash, NEW_PASSWORD, SECRET));
        assert!(!compare_password(&hash, OLD_PASSWORD, SECRET));

        // Email got verified by the reset.
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

        // Token burned.
        let token = ctx
            .tokens
            .memory
            .data
            .lock()
            .unwrap()
            .get(&ctx.reset_id)
            .cloned()
            .unwrap();
        assert!(token.is_consumed());

        // Refresh token revoked.
        let rt = ctx
            .refresh
            .memory
            .data
            .lock()
            .unwrap()
            .get(&ctx.refresh_id)
            .cloned()
            .unwrap();
        assert!(rt.revoked_at().is_some());
    }

    #[tokio::test]
    async fn fails_on_wrong_token_and_leaves_password_unchanged() {
        let ctx = setup(false, false);
        let before = current_hash(&ctx);

        let err = ctx
            .use_case
            .execute(command(&ctx, "not-the-token"))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));

        assert_eq!(current_hash(&ctx), before);
        let rt = ctx
            .refresh
            .memory
            .data
            .lock()
            .unwrap()
            .get(&ctx.refresh_id)
            .cloned()
            .unwrap();
        assert!(rt.revoked_at().is_none());
    }

    #[tokio::test]
    async fn fails_on_expired_token() {
        let ctx = setup(false, true);
        let err = ctx
            .use_case
            .execute(command(&ctx, RAW_TOKEN))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn fails_on_consumed_token() {
        let ctx = setup(true, false);
        let err = ctx
            .use_case
            .execute(command(&ctx, RAW_TOKEN))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn fails_on_malformed_reset_id() {
        let ctx = setup(false, false);
        let mut cmd = command(&ctx, RAW_TOKEN);
        cmd.password_reset_id = "not-a-uuid".to_string();

        let err = ctx.use_case.execute(cmd).await.unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }
}
