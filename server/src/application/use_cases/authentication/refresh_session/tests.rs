#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use chrono::{DateTime, Duration, Utc};

    use crate::{
        application::use_cases::authentication::{
            refresh_session::{command::Command, use_case::UseCase},
            session_policy::SESSION_IDLE_TTL_MINUTES,
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
        client: OAuthClient,
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
            vec!["openid".to_string(), "email".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
    }

    // An authenticated session keyed by the hash of RAW_SESSION_CODE. Both deadlines
    // are overridable so the idle window and the ceiling can be driven independently —
    // the interaction between them is the whole point of this use case.
    fn make_session(
        tenant_id: Uuid,
        client: &OAuthClient,
        user_id: Uuid,
        authenticated: bool,
        expires_at: DateTime<Utc>,
        absolute_expires_at: DateTime<Utc>,
    ) -> AuthSession {
        let mut session = AuthSession::new(
            tenant_id,
            sha256_hex(RAW_SESSION_CODE),
            client.id(),
            "https://app.example.com/callback".to_string(),
            vec!["openid".to_string()],
            "xyz".to_string(),
            "challenge".to_string(),
            "S256".to_string(),
            Some("n-1".to_string()),
            expires_at,
            absolute_expires_at,
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

        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "Test Tenant".to_string(),
            None,
            "secret".to_string(),
            true,
            true,
            true,
            Uuid::now_v7(),
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
        let user_id = user.id();
        users.memory.data.lock().unwrap().insert(user.id(), user);

        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_oauth_client_repository(clients)
            .with_auth_session_repository(sessions.clone());

        Ctx {
            use_case: UseCase::new(Arc::new(factory)),
            sessions,
            client,
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

    fn make_command() -> Command {
        Command {
            session_code: RAW_SESSION_CODE.to_string(),
            client_id: client_id().to_string(),
        }
    }

    // Reads the single stored session back out.
    fn stored(ctx: &Ctx) -> AuthSession {
        ctx.sessions
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .next()
            .cloned()
            .unwrap()
    }

    #[tokio::test]
    async fn slides_the_idle_deadline_forward_and_rotates_the_token() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        let old_deadline = Utc::now() + Duration::minutes(1);
        seed(
            &ctx,
            make_session(
                tenant_id,
                &ctx.client,
                ctx.user_id,
                true,
                old_deadline,
                Utc::now() + Duration::minutes(30),
            ),
        );

        let response = ctx.use_case.execute(make_command()).await.unwrap();

        // The idle window moved out to roughly now + SESSION_IDLE_TTL_MINUTES.
        assert!(response.session_expires_at > old_deadline);
        let granted = response.session_expires_at - Utc::now();
        assert!(granted.num_seconds() > (SESSION_IDLE_TTL_MINUTES * 60) - 10);
        assert!(granted.num_seconds() <= SESSION_IDLE_TTL_MINUTES * 60);

        // A brand-new token came back, and it is the one now stored.
        assert_ne!(response.session_code, RAW_SESSION_CODE);
        assert_eq!(
            stored(&ctx).session_token_hash(),
            sha256_hex(&response.session_code)
        );
    }

    #[tokio::test]
    async fn the_old_session_code_dies_on_rotation() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        seed(
            &ctx,
            make_session(
                tenant_id,
                &ctx.client,
                ctx.user_id,
                true,
                Utc::now() + Duration::minutes(1),
                Utc::now() + Duration::minutes(30),
            ),
        );

        ctx.use_case.execute(make_command()).await.unwrap();

        // Replaying the pre-rotation code no longer resolves a session.
        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn clamps_the_new_deadline_to_the_absolute_ceiling() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        // Only 2 minutes left on the ceiling — far less than a full idle window.
        let ceiling = Utc::now() + Duration::minutes(2);
        seed(
            &ctx,
            make_session(
                tenant_id,
                &ctx.client,
                ctx.user_id,
                true,
                Utc::now() + Duration::minutes(1),
                ceiling,
            ),
        );

        let response = ctx.use_case.execute(make_command()).await.unwrap();

        // Refreshing bought time, but only up to the ceiling — not the full 10 minutes.
        assert_eq!(response.session_expires_at, ceiling);
        assert_eq!(response.session_absolute_expires_at, ceiling);
    }

    #[tokio::test]
    async fn refuses_once_the_absolute_ceiling_has_passed() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        // Still inside the idle window, but the ceiling is behind us: this is exactly the
        // case a naive sliding window would renew forever.
        seed(
            &ctx,
            make_session(
                tenant_id,
                &ctx.client,
                ctx.user_id,
                true,
                Utc::now() + Duration::minutes(5),
                Utc::now() - Duration::seconds(1),
            ),
        );

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn rejects_an_expired_session() {
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
                Utc::now() + Duration::minutes(30),
            ),
        );

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn rejects_a_session_already_spent_for_an_authorization_code() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        let mut session = make_session(
            tenant_id,
            &ctx.client,
            ctx.user_id,
            true,
            Utc::now() + Duration::minutes(5),
            Utc::now() + Duration::minutes(30),
        );
        // authorize_session burns the session this way once it has minted a code.
        session.expire_now(Utc::now());
        seed(&ctx, session);

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn rejects_an_unauthenticated_session() {
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
                Utc::now() + Duration::minutes(30),
            ),
        );

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn rejects_an_unknown_session_code() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        // Nothing seeded.

        let err = ctx.use_case.execute(make_command()).await.unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn refreshing_keeps_a_pending_mfa_gate_closed() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id);
        let mut session = make_session(
            tenant_id,
            &ctx.client,
            ctx.user_id,
            true,
            Utc::now() + Duration::minutes(1),
            Utc::now() + Duration::minutes(30),
        );
        session.require_mfa();
        seed(&ctx, session);

        // Refreshing while mid-MFA is the main use of this endpoint, so it must succeed…
        ctx.use_case.execute(make_command()).await.unwrap();

        // …but it must not be a way to walk past the second factor.
        assert!(stored(&ctx).is_mfa_pending());
    }
}
