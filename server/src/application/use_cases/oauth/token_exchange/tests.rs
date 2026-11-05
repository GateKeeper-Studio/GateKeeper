#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth::token_exchange::{command::Command, use_case::UseCase},
        domain::{
            entities::{
                authorization_code::AuthorizationCode,
                group::Group,
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                role::Role,
                signing_key::{SigningAlgorithm, SigningKey},
            },
            errors::DomainError,
        },
        testing::{
            fake_audit_logger::FakeAuditLogger,
            in_memory_repositories::{
                in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
                in_memory_group_repository::InMemoryGroupRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
                in_memory_role_repository::InMemoryRoleRepository,
                in_memory_signing_key_repository::InMemorySigningKeyRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            oauth_fakes::FakeTokenSigner,
        },
        utils::oauth_crypto::{pkce_challenge_s256, sha256_hex},
    };

    const VERIFIER: &str = "the-secret-code-verifier-kept-by-the-client";
    const RAW_CODE: &str = "raw-authorization-code-value";

    fn client_id() -> Uuid {
        Uuid::parse_str("01890000-0000-7000-8000-000000000abc").unwrap()
    }

    struct Ctx {
        use_case: UseCase,
        codes: Arc<InMemoryAuthorizationCodeRepository>,
        refresh: Arc<InMemoryRefreshTokenRepository>,
        audit: FakeAuditLogger,
        tenant_id: Uuid,
        user_id: Uuid,
    }

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::new(
            tenant_id,
            client_id(),
            None,
            ClientType::Public,
            "Test App".to_string(),
            vec!["https://app.example.com/callback".to_string()],
            vec![],
            vec!["openid".to_string(), "email".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
    }

    fn make_code(tenant_id: Uuid, user_id: Uuid, client_id: Uuid) -> AuthorizationCode {
        AuthorizationCode::new(
            tenant_id,
            sha256_hex(RAW_CODE),
            client_id.to_string(),
            user_id,
            "https://app.example.com/callback".to_string(),
            vec!["openid".to_string(), "email".to_string()],
            pkce_challenge_s256(VERIFIER),
            "S256".to_string(),
            "nonce-1".to_string(),
            Utc::now() + Duration::seconds(60),
        )
    }

    fn setup() -> Ctx {
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let client = make_client(tenant_id);
        let client_id = client.id();
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client);

        let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
        let code = make_code(tenant_id, user_id, client_id);
        codes.memory.data.lock().unwrap().insert(code.id(), code);

        let keys = Arc::new(InMemorySigningKeyRepository::new());
        let key = SigningKey::new(
            tenant_id,
            "kid-1".to_string(),
            SigningAlgorithm::RS256,
            "{}".to_string(),
            "secret".to_string(),
        );
        keys.memory.data.lock().unwrap().insert(key.id(), key);

        let refresh = Arc::new(InMemoryRefreshTokenRepository::new());

        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_oauth_client_repository(clients)
        .with_authorization_code_repository(codes.clone())
        .with_signing_key_repository(keys)
        .with_refresh_token_repository(refresh.clone());

        let audit = FakeAuditLogger::new();
        let use_case = UseCase::new(
            Arc::new(factory),
            Arc::new(FakeTokenSigner::new()),
            "https://auth.test".to_string(),
            Arc::new(audit.clone()),
        );

        Ctx {
            use_case,
            codes,
            refresh,
            audit,
            tenant_id,
            user_id,
        }
    }

    fn make_command(tenant_id: Uuid) -> Command {
        Command {
            tenant_id,
            client_id: client_id().to_string(),
            client_secret: None,
            code: RAW_CODE.to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            code_verifier: VERIFIER.to_string(),
        }
    }

    #[tokio::test]
    async fn happy_path_issues_three_tokens_and_consumes_code() {
        let ctx = setup();

        let response = ctx
            .use_case
            .execute(make_command(ctx.tenant_id))
            .await
            .unwrap();

        assert!(!ctx.refresh.memory.data.lock().unwrap().is_empty());
        assert!(response.id_token.is_some()); // openid scope present
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 900);
        assert!(response.access_token.contains(&ctx.user_id.to_string()));

        // The code is now consumed.
        let codes = ctx.codes.memory.data.lock().unwrap();
        assert!(codes.values().next().unwrap().is_consumed());

        // Audit: a `token.issued` event was emitted for the sign-in (WI-02).
        assert_eq!(
            ctx.audit
                .count_action(crate::domain::entities::audit_action::TOKEN_ISSUED),
            1
        );
    }

    #[tokio::test]
    async fn second_redemption_of_the_same_code_fails() {
        let ctx = setup();

        ctx.use_case
            .execute(make_command(ctx.tenant_id))
            .await
            .unwrap();
        let err = ctx
            .use_case
            .execute(make_command(ctx.tenant_id))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidGrant(_)));
    }

    #[tokio::test]
    async fn rejects_bad_pkce_verifier() {
        let ctx = setup();

        let mut command = make_command(ctx.tenant_id);
        command.code_verifier = "wrong-verifier".to_string();

        let err = ctx.use_case.execute(command).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidGrant(_)));
    }

    #[tokio::test]
    async fn rejects_redirect_uri_mismatch() {
        let ctx = setup();

        let mut command = make_command(ctx.tenant_id);
        command.redirect_uri = "https://app.example.com/other".to_string();

        let err = ctx.use_case.execute(command).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidGrant(_)));
    }

    #[tokio::test]
    async fn no_id_token_without_openid_scope() {
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let client = make_client(tenant_id);
        let client_id = client.id();
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client);

        // A code that only granted "email" — no openid.
        let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
        let code = AuthorizationCode::new(
            tenant_id,
            sha256_hex(RAW_CODE),
            client_id.to_string(),
            user_id,
            "https://app.example.com/callback".to_string(),
            vec!["email".to_string()],
            pkce_challenge_s256(VERIFIER),
            "S256".to_string(),
            "".to_string(),
            Utc::now() + Duration::seconds(60),
        );
        codes.memory.data.lock().unwrap().insert(code.id(), code);

        let keys = Arc::new(InMemorySigningKeyRepository::new());
        let key = SigningKey::new(
            tenant_id,
            "kid-1".to_string(),
            SigningAlgorithm::RS256,
            "{}".to_string(),
            "secret".to_string(),
        );
        keys.memory.data.lock().unwrap().insert(key.id(), key);

        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_oauth_client_repository(clients)
        .with_authorization_code_repository(codes)
        .with_signing_key_repository(keys)
        .with_refresh_token_repository(Arc::new(InMemoryRefreshTokenRepository::new()));

        let use_case = UseCase::new(
            Arc::new(factory),
            Arc::new(FakeTokenSigner::new()),
            "https://auth.test".to_string(),
            Arc::new(FakeAuditLogger::new()),
        );

        let response = use_case.execute(make_command(tenant_id)).await.unwrap();
        assert!(response.id_token.is_none());
    }

    #[tokio::test]
    async fn access_and_id_tokens_carry_user_roles_and_groups() {
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let client = make_client(tenant_id);
        let client_id = client.id();
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client);

        let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
        let code = make_code(tenant_id, user_id, client_id);
        codes.memory.data.lock().unwrap().insert(code.id(), code);

        let keys = Arc::new(InMemorySigningKeyRepository::new());
        let key = SigningKey::new(
            tenant_id,
            "kid-1".to_string(),
            SigningAlgorithm::RS256,
            "{}".to_string(),
            "secret".to_string(),
        );
        keys.memory.data.lock().unwrap().insert(key.id(), key);

        let users = Arc::new(InMemoryUserRepository::new());

        // Seed a committed role + group assignment for this user.
        let roles = Arc::new(InMemoryRoleRepository::new(users.clone()));
        let role = Role::new(tenant_id, "admin".to_string(), None);
        let role_id = role.id();
        roles.memory.data.lock().unwrap().insert(role.id(), role);
        roles.assignments.lock().unwrap().insert((role_id, user_id));

        let groups = Arc::new(InMemoryGroupRepository::new(users.clone()));
        let group = Group::new(tenant_id, "engineering".to_string(), None);
        let group_id = group.id();
        groups.memory.data.lock().unwrap().insert(group.id(), group);
        groups.members.lock().unwrap().insert((group_id, user_id));

        let signer = FakeTokenSigner::new();
        let signed = signer.signed_claims();

        let factory =
            InMemoryUnitOfWorkFactory::new(users, Arc::new(InMemoryTenantRepository::new()))
                .with_oauth_client_repository(clients)
                .with_authorization_code_repository(codes)
                .with_signing_key_repository(keys)
                .with_refresh_token_repository(Arc::new(InMemoryRefreshTokenRepository::new()))
                .with_role_repository(roles)
                .with_group_repository(groups);

        let use_case = UseCase::new(
            Arc::new(factory),
            Arc::new(signer),
            "https://auth.test".to_string(),
            Arc::new(FakeAuditLogger::new()),
        );

        let response = use_case.execute(make_command(tenant_id)).await.unwrap();
        assert!(response.id_token.is_some()); // openid scope ⇒ id_token issued too

        // Both the access token and the id_token must carry the same role/group names.
        let signed = signed.lock().unwrap();
        assert_eq!(signed.len(), 2);
        for claims in signed.iter() {
            assert_eq!(claims["roles"], serde_json::json!(["admin"]));
            assert_eq!(claims["groups"], serde_json::json!(["engineering"]));
        }
    }

    // Confidential clients: the secret is the client's proof of identity, so the
    // exchange must accept only the CURRENT secret. These run the real rotate use
    // case against the same store the exchange reads, which is what caught a
    // rotation that returned a new secret but never persisted its hash.
    mod confidential_client {
        use super::*;

        use crate::{
            application::use_cases::oauth_client::rotate_oauth_client_secret::{
                command::Command as RotateCommand, use_case::UseCase as RotateUseCase,
            },
            domain::entities::tenant::Tenant,
            utils::password_hasher::hash_password,
        };

        const PEPPER: &str = "tenant-pepper";
        const ORIGINAL_SECRET: &str = "the-original-client-secret";

        struct Ctx {
            exchange: UseCase,
            rotate: RotateUseCase,
            codes: Arc<InMemoryAuthorizationCodeRepository>,
            tenant_id: Uuid,
            user_id: Uuid,
            client_db_id: Uuid,
        }

        fn setup() -> Ctx {
            let tenant = Tenant::new(
                "T".to_string(),
                None,
                PEPPER.to_string(),
                true,
                true,
                true,
                Uuid::now_v7(),
            );
            let tenant_id = tenant.id();
            let user_id = Uuid::now_v7();

            let tenants = Arc::new(InMemoryTenantRepository::new());
            tenants
                .memory
                .data
                .lock()
                .unwrap()
                .insert(tenant_id, tenant);

            let client = OAuthClient::new(
                tenant_id,
                client_id(),
                Some(hash_password(ORIGINAL_SECRET, PEPPER).unwrap()),
                ClientType::Confidential,
                "Test App".to_string(),
                vec!["https://app.example.com/callback".to_string()],
                vec![],
                vec!["openid".to_string(), "email".to_string()],
                vec!["authorization_code".to_string()],
                true,
                true,
                vec!["en".to_string()],
                "en".to_string(),
                MfaPolicy::default(),
            );
            let client_db_id = client.id();
            let clients = Arc::new(InMemoryOAuthClientRepository::new());
            clients
                .memory
                .data
                .lock()
                .unwrap()
                .insert(client_db_id, client);

            let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
            let keys = Arc::new(InMemorySigningKeyRepository::new());
            let key = SigningKey::new(
                tenant_id,
                "kid-1".to_string(),
                SigningAlgorithm::RS256,
                "{}".to_string(),
                "secret".to_string(),
            );
            keys.memory.data.lock().unwrap().insert(key.id(), key);

            let factory = Arc::new(
                InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenants)
                    .with_oauth_client_repository(clients)
                    .with_authorization_code_repository(codes.clone())
                    .with_signing_key_repository(keys)
                    .with_refresh_token_repository(Arc::new(InMemoryRefreshTokenRepository::new())),
            );

            Ctx {
                exchange: UseCase::new(
                    factory.clone(),
                    Arc::new(FakeTokenSigner::new()),
                    "https://auth.test".to_string(),
                    Arc::new(FakeAuditLogger::new()),
                ),
                rotate: RotateUseCase::new(factory),
                codes,
                tenant_id,
                user_id,
                client_db_id,
            }
        }

        // Each successful exchange consumes its code, so tests that redeem more
        // than once issue a fresh one first.
        fn issue_code(ctx: &Ctx) {
            let code = make_code(ctx.tenant_id, ctx.user_id, ctx.client_db_id);
            ctx.codes.memory.data.lock().unwrap().clear();
            ctx.codes
                .memory
                .data
                .lock()
                .unwrap()
                .insert(code.id(), code);
        }

        fn command_with_secret(ctx: &Ctx, secret: Option<&str>) -> Command {
            Command {
                client_secret: secret.map(str::to_string),
                ..make_command(ctx.tenant_id)
            }
        }

        #[tokio::test]
        async fn accepts_the_correct_secret() {
            let ctx = setup();
            issue_code(&ctx);

            let response = ctx
                .exchange
                .execute(command_with_secret(&ctx, Some(ORIGINAL_SECRET)))
                .await
                .unwrap();

            assert_eq!(response.token_type, "Bearer");
        }

        #[tokio::test]
        async fn rejects_a_wrong_secret_without_consuming_the_code() {
            let ctx = setup();
            issue_code(&ctx);

            let err = ctx
                .exchange
                .execute(command_with_secret(&ctx, Some("not-the-secret")))
                .await
                .unwrap_err();

            assert!(matches!(err, DomainError::InvalidClient(_)));
            let codes = ctx.codes.memory.data.lock().unwrap();
            assert!(!codes.values().next().unwrap().is_consumed());
        }

        #[tokio::test]
        async fn rejects_a_missing_secret() {
            let ctx = setup();
            issue_code(&ctx);

            let err = ctx
                .exchange
                .execute(command_with_secret(&ctx, None))
                .await
                .unwrap_err();

            assert!(matches!(err, DomainError::InvalidClient(_)));
        }

        #[tokio::test]
        async fn after_rotation_only_the_new_secret_is_accepted() {
            let ctx = setup();

            let rotated = ctx
                .rotate
                .execute(RotateCommand {
                    tenant_id: ctx.tenant_id,
                    id: ctx.client_db_id,
                })
                .await
                .unwrap();

            issue_code(&ctx);
            let err = ctx
                .exchange
                .execute(command_with_secret(&ctx, Some(ORIGINAL_SECRET)))
                .await
                .unwrap_err();
            assert!(
                matches!(err, DomainError::InvalidClient(_)),
                "the pre-rotation secret must stop working"
            );

            ctx.exchange
                .execute(command_with_secret(&ctx, Some(&rotated.client_secret)))
                .await
                .expect("the secret returned by rotation must be accepted");
        }
    }
}
