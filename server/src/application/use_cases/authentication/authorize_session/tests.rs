#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use chrono::{Duration, Utc};

    use crate::{
        application::use_cases::authentication::authorize_session::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                auth_session::AuthSession,
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_auth_session_repository::InMemoryAuthSessionRepository,
                in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
                in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            user_factory::generate_user,
        },
        utils::oauth_crypto::sha256_hex,
    };

    const RAW_SESSION_CODE: &str = "raw-session-code-abc123";

    struct Ctx {
        use_case: UseCase,
        sessions: Arc<InMemoryAuthSessionRepository>,
        codes: Arc<InMemoryAuthorizationCodeRepository>,
        consents: Arc<InMemoryConsentGrantRepository>,
        client: OAuthClient,
        email: String,
        user_id: Uuid,
    }

    fn client_id() -> Uuid {
        Uuid::parse_str("01890000-0000-7000-8000-000000000abc").unwrap()
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

    // Builds an authenticated session for (tenant, client, user), keyed by the hash
    // of RAW_SESSION_CODE. `authenticated` and `expires_at` are overridable so the
    // failure paths can be exercised.
    fn make_session(
        tenant_id: Uuid,
        client: &OAuthClient,
        user_id: Uuid,
        authenticated: bool,
        expires_at: chrono::DateTime<Utc>,
    ) -> AuthSession {
        let mut session = AuthSession::new(
            tenant_id,
            sha256_hex(RAW_SESSION_CODE),
            client.id(),
            "https://app.example.com/callback".to_string(),
            vec!["openid".to_string(), "email".to_string()],
            "xyz".to_string(),
            "challenge".to_string(),
            "S256".to_string(),
            Some("n-1".to_string()),
            expires_at,
            // These tests never exercise the ceiling; keep it well clear of the idle
            // deadline so it is never the thing under test.
            expires_at + Duration::hours(1),
        );
        if authenticated {
            session.authenticate(user_id, Utc::now());
        }
        session
    }

    fn setup(tenant_id: Uuid) -> Ctx {
        let client = make_client(tenant_id);

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let sessions = Arc::new(InMemoryAuthSessionRepository::new());
        let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
        let consents = Arc::new(InMemoryConsentGrantRepository::new());

        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "Test Tenant".to_string(),
            None,
            "secret".to_string(),
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

        let users = Arc::new(InMemoryUserRepository::new());
        let user = generate_user(tenant_id);
        let email = user.email().to_string();
        let user_id = user.id();
        users.memory.data.lock().unwrap().insert(user.id(), user);

        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_oauth_client_repository(clients)
            .with_auth_session_repository(sessions.clone())
            .with_authorization_code_repository(codes.clone())
            .with_consent_grant_repository(consents.clone());

        Ctx {
            use_case: UseCase::new(Arc::new(factory)),
            sessions,
            codes,
            consents,
            client,
            email,
            user_id,
        }
    }

    fn seed(ctx: &Ctx, session: AuthSession) {
        ctx.sessions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(session.id(), session);
    }

    fn make_command(email: &str) -> Command {
        Command {
            session_code: RAW_SESSION_CODE.to_string(),
            client_id: client_id().to_string(),
            email: email.to_string(),
        }
    }

    #[tokio::test]
    async fn issues_code_from_frozen_session_and_persists_consent() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        seed(
            &ctx,
            make_session(
                tenant_id,
                &ctx.client,
                ctx.user_id,
                true,
                Utc::now() + Duration::minutes(5),
            ),
        );

        let response = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap();

        // Values come from the frozen session, not the request body.
        assert_eq!(response.state, "xyz");
        assert_eq!(response.redirect_uri, "https://app.example.com/callback");
        assert!(!response.authorization_code.is_empty());

        // The code is persisted as a hash, never raw, with a ~60s expiry.
        let stored = ctx.codes.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let code = stored.values().next().unwrap();
        assert_eq!(code.code_hash(), sha256_hex(&response.authorization_code));
        let ttl = code.expires_at() - code.created_at();
        assert!(ttl.num_seconds() > 0 && ttl.num_seconds() <= 60);

        // Consent now covers the frozen scopes.
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
    async fn burns_the_session_so_the_code_cannot_be_replayed() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        seed(
            &ctx,
            make_session(
                tenant_id,
                &ctx.client,
                ctx.user_id,
                true,
                Utc::now() + Duration::minutes(5),
            ),
        );

        ctx.use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap();

        // The session is now expired; a second exchange fails.
        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
        // Only the first code was minted.
        assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn rejects_unknown_session_code() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        // No session seeded.

        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn rejects_expired_session() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        seed(
            &ctx,
            make_session(
                tenant_id,
                &ctx.client,
                ctx.user_id,
                true,
                Utc::now() - Duration::seconds(1),
            ),
        );

        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
        assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn rejects_unauthenticated_session() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        seed(
            &ctx,
            make_session(
                tenant_id,
                &ctx.client,
                ctx.user_id,
                false,
                Utc::now() + Duration::minutes(5),
            ),
        );

        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn rejects_session_with_pending_mfa() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        let mut session = make_session(
            tenant_id,
            &ctx.client,
            ctx.user_id,
            true,
            Utc::now() + Duration::minutes(5),
        );
        session.require_mfa();
        seed(&ctx, session);

        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
        assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn issues_code_once_mfa_is_verified() {
        use crate::domain::entities::mfa_method::MfaMethod;

        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        let mut session = make_session(
            tenant_id,
            &ctx.client,
            ctx.user_id,
            true,
            Utc::now() + Duration::minutes(5),
        );
        session.require_mfa();
        session.verify_mfa(&MfaMethod::Totp, Utc::now());
        seed(&ctx, session);

        let response = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap();
        assert!(!response.authorization_code.is_empty());
    }

    #[tokio::test]
    async fn rejects_email_not_matching_the_bound_user() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        seed(
            &ctx,
            make_session(
                tenant_id,
                &ctx.client,
                ctx.user_id,
                true,
                Utc::now() + Duration::minutes(5),
            ),
        );

        let err = ctx
            .use_case
            .execute(make_command("someone-else@example.com"))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
        assert_eq!(ctx.codes.memory.data.lock().unwrap().len(), 0);
    }
}
