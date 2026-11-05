#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::use_cases::authentication::change_password::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                password_change_token::PasswordChangeToken,
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
                in_memory_password_change_token_repository::InMemoryPasswordChangeTokenRepository,
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
    const TEMP_PASSWORD: &str = "temp-password-123";
    const NEW_PASSWORD: &str = "new-password-456";
    const RAW_CODE: &str = "raw-change-code-value";

    struct Ctx {
        use_case: UseCase,
        users: Arc<InMemoryUserRepository>,
        tokens: Arc<InMemoryPasswordChangeTokenRepository>,
        refresh: Arc<InMemoryRefreshTokenRepository>,
        user_id: Uuid,
        token_id: Uuid,
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

        // Active, verified user whose credentials still require a password change.
        let users = Arc::new(InMemoryUserRepository::new());
        let mut user = User::new(tenant_id, "user@example.com".to_string(), None, None);
        user.verify_email();
        let temp_hash = hash_password(TEMP_PASSWORD, SECRET).unwrap();
        user.set_credentials(UserCredentials::new(
            user.id(),
            temp_hash,
            "argon2".to_string(),
            true,
        ));
        let user_id = user.id();
        users.memory.data.lock().unwrap().insert(user.id(), user);

        let tokens = Arc::new(InMemoryPasswordChangeTokenRepository::new());
        let expires_at = if expired {
            Utc::now() - Duration::hours(1)
        } else {
            Utc::now() + Duration::minutes(15)
        };
        let mut token =
            PasswordChangeToken::new(tenant_id, user_id, sha256_hex(RAW_CODE), expires_at);
        if consumed {
            token.consume(Utc::now());
        }
        let token_id = token.id();
        tokens.memory.data.lock().unwrap().insert(token.id(), token);

        // A live refresh token that must be revoked by a successful change.
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
            .with_password_change_token_repository(tokens.clone())
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
            token_id,
            refresh_id,
        }
    }

    fn command(ctx: &Ctx, raw_code: &str) -> Command {
        Command {
            client_id: CLIENT_ID.to_string(),
            user_id: ctx.user_id.to_string(),
            change_password_code: raw_code.to_string(),
            new_password: NEW_PASSWORD.to_string(),
        }
    }

    fn current_credentials(ctx: &Ctx) -> (String, bool) {
        let users = ctx.users.memory.data.lock().unwrap();
        let creds = users
            .get(&ctx.user_id)
            .unwrap()
            .credentials()
            .unwrap()
            .clone();
        (
            creds.password_hash().to_string(),
            creds.should_change_password(),
        )
    }

    // An external-login-only client has no password path to force a change on, so a
    // code minted before the flag was turned on is refused rather than redeemed.
    #[tokio::test]
    async fn refuses_for_an_external_login_only_client() {
        let ctx = setup_ext(false, false, true);
        let before = current_credentials(&ctx);

        let err = ctx
            .use_case
            .execute(command(&ctx, RAW_CODE))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::AccessDenied(_)));
        assert_eq!(current_credentials(&ctx), before);
    }

    #[tokio::test]
    async fn changes_password_clears_flag_consumes_code_and_revokes_sessions() {
        let ctx = setup(false, false);

        ctx.use_case.execute(command(&ctx, RAW_CODE)).await.unwrap();

        // Password is now the new one (and no longer the temporary one).
        let (hash, should_change) = current_credentials(&ctx);
        assert!(compare_password(&hash, NEW_PASSWORD, SECRET));
        assert!(!compare_password(&hash, TEMP_PASSWORD, SECRET));
        // Forced-change flag cleared.
        assert!(!should_change);

        // Change code burned.
        let token = ctx
            .tokens
            .memory
            .data
            .lock()
            .unwrap()
            .get(&ctx.token_id)
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
    async fn fails_on_wrong_code_and_leaves_password_unchanged() {
        let ctx = setup(false, false);
        let before = current_credentials(&ctx);

        let err = ctx
            .use_case
            .execute(command(&ctx, "not-the-code"))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));

        let after = current_credentials(&ctx);
        assert_eq!(after.0, before.0);
        assert!(after.1, "forced-change flag must remain set on failure");
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
    async fn fails_on_expired_code() {
        let ctx = setup(false, true);
        let err = ctx
            .use_case
            .execute(command(&ctx, RAW_CODE))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn fails_on_consumed_code() {
        let ctx = setup(true, false);
        let err = ctx
            .use_case
            .execute(command(&ctx, RAW_CODE))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn fails_on_malformed_user_id() {
        let ctx = setup(false, false);
        let mut cmd = command(&ctx, RAW_CODE);
        cmd.user_id = "not-a-uuid".to_string();

        let err = ctx.use_case.execute(cmd).await.unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }
}
