#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use chrono::Utc;
    use serde_json::json;

    use crate::{
        application::use_cases::authentication::login::{command::Command, use_case::UseCase},
        domain::{
            entities::{
                mfa_enrollment::MfaEnrollment,
                mfa_method::MfaMethod,
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::{
            fake_webauthn_service::{FAKE_CREDENTIAL_ID, FakeWebauthnService},
            in_memory_repositories::{
                in_memory_auth_session_repository::InMemoryAuthSessionRepository,
                in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            templated_email_support::recording_templated_email_sender,
            user_factory::{TEST_PASSWORD, TEST_PASSWORD_SECRET, generate_user},
        },
        utils::oauth_crypto::sha256_hex,
    };

    struct Ctx {
        use_case: UseCase,
        client: OAuthClient,
        sessions: Arc<InMemoryAuthSessionRepository>,
        users: Arc<InMemoryUserRepository>,
        mfa_enrollments: Arc<InMemoryMfaEnrollmentRepository>,
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

    fn make_client_with_mfa_policy(tenant_id: Uuid, mfa_policy: MfaPolicy) -> OAuthClient {
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
            mfa_policy,
        )
    }

    // `verified` toggles whether the seeded user has confirmed their email.
    fn setup(tenant_id: Uuid, verified: bool) -> Ctx {
        let client = make_client(tenant_id);
        setup_with_client(tenant_id, verified, client)
    }

    fn setup_with_client(tenant_id: Uuid, verified: bool, client: OAuthClient) -> Ctx {
        setup_with_client_and_risk(
            tenant_id,
            verified,
            client,
            Arc::new(crate::testing::fake_risk_engine::FakeRiskEngine::allow()),
        )
    }

    fn setup_with_client_and_risk(
        tenant_id: Uuid,
        verified: bool,
        client: OAuthClient,
        risk_engine: Arc<dyn crate::application::ports::risk_engine::RiskEngine>,
    ) -> Ctx {
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
            TEST_PASSWORD_SECRET.to_string(),
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
        let mut user = generate_user(tenant_id);
        if verified {
            user.verify_email();
        }
        let email = user.email().to_string();
        let user_id = user.id();
        users.memory.data.lock().unwrap().insert(user.id(), user);

        let mfa_enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
        let factory = InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_oauth_client_repository(clients)
            .with_auth_session_repository(sessions.clone())
            .with_mfa_enrollment_repository(mfa_enrollments.clone());

        Ctx {
            use_case: UseCase::new(
                Arc::new(factory),
                recording_templated_email_sender().0,
                Arc::new(FakeWebauthnService::new()),
                risk_engine,
                crate::testing::password_hasher::test_password_hasher(),
            ),
            client,
            sessions,
            users,
            mfa_enrollments,
            email,
            user_id,
        }
    }

    fn make_command(email: &str) -> Command {
        Command {
            email: email.to_string(),
            password: TEST_PASSWORD.to_string(),
            client_id: client_id().to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            response_type: "code".to_string(),
            scope: "openid email".to_string(),
            state: "xyz".to_string(),
            code_challenge: "challenge".to_string(),
            code_challenge_method: "S256".to_string(),
            nonce: Some("n-1".to_string()),
            mfa_methods: None,
            mfa_default_method: None,
            ip_address: None,
            user_agent: None,
        }
    }

    fn required_mfa_policy() -> MfaPolicy {
        MfaPolicy {
            required: true,
            email_enabled: true,
            totp_enabled: true,
            webauthn_enabled: true,
        }
    }

    fn seed_totp(ctx: &Ctx) -> Uuid {
        let mut enrollment =
            MfaEnrollment::new_totp(ctx.client.tenant_id(), ctx.user_id, "encrypted".to_string());
        enrollment.confirm(Utc::now());
        let id = enrollment.id();
        ctx.mfa_enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(id, enrollment);
        id
    }

    fn seed_webauthn(ctx: &Ctx) {
        let enrollment = MfaEnrollment::new_webauthn(
            ctx.client.tenant_id(),
            ctx.user_id,
            json!({ "fake_passkey": true, "cred_id": FAKE_CREDENTIAL_ID, "counter": 0 }),
            FAKE_CREDENTIAL_ID.to_string(),
            Some("Test key".to_string()),
        );
        ctx.mfa_enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);
    }

    // An external-login-only client refuses password sign-in outright. The command
    // below carries the user's REAL password and a verified account, so a pass here
    // would mean the gate runs after authentication instead of before it — this asserts
    // the credentials are never spent, and no session is opened.
    #[tokio::test]
    async fn refuses_password_login_for_an_external_login_only_client() {
        let tenant_id = Uuid::now_v7();
        let mut client = make_client(tenant_id);
        client.set_external_login_only(true);
        let ctx = setup_with_client(tenant_id, true, client);

        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::AccessDenied(_)));
        assert!(ctx.sessions.memory.data.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn opens_authenticated_session_and_freezes_request() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let response = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap();

        assert!(!response.session_code.is_empty());
        assert_eq!(response.user_id, ctx.user_id.to_string());
        assert!(response.mfa_type.is_none());

        // Exactly one session, bound to the user, with only the hash stored and the
        // request frozen verbatim.
        let stored = ctx.sessions.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let session = stored.values().next().unwrap();
        assert_eq!(
            session.session_token_hash(),
            sha256_hex(&response.session_code)
        );
        assert!(session.is_authenticated());
        assert_eq!(session.user_id(), Some(ctx.user_id));
        assert_eq!(session.pending_client_id(), ctx.client.id());
        assert_eq!(
            session.pending_redirect_uri(),
            "https://app.example.com/callback"
        );
        assert_eq!(
            session.pending_scopes(),
            &["openid".to_string(), "email".to_string()]
        );
        assert_eq!(session.pending_state(), "xyz");
        assert_eq!(session.pending_nonce(), Some("n-1"));
    }

    #[tokio::test]
    async fn logs_in_when_email_case_differs() {
        // The stored email is canonical (lowercase); a login using a different case
        // must still resolve the same account.
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let response = ctx
            .use_case
            .execute(make_command(&ctx.email.to_uppercase()))
            .await
            .unwrap();

        assert!(!response.session_code.is_empty());
        assert_eq!(response.user_id, ctx.user_id.to_string());
    }

    #[tokio::test]
    async fn rejects_wrong_password() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let mut command = make_command(&ctx.email);
        command.password = "not-the-password".to_string();

        let err = ctx.use_case.execute(command).await.unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
        assert_eq!(ctx.sessions.memory.data.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn rejects_unknown_user() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let err = ctx
            .use_case
            .execute(make_command("nobody@example.com"))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn rejects_unconfirmed_email() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, false); // user not verified

        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::EmailNotConfirmed(_)));
        // No session is opened for an unconfirmed account.
        assert_eq!(ctx.sessions.memory.data.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn rejects_mismatched_redirect_uri() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let mut command = make_command(&ctx.email);
        command.redirect_uri = "https://evil.example.com/callback".to_string();

        let err = ctx.use_case.execute(command).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn rejects_non_s256_pkce() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup(tenant_id, true);

        let mut command = make_command(&ctx.email);
        command.code_challenge_method = "plain".to_string();

        let err = ctx.use_case.execute(command).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn selects_default_mfa_hint_when_available() {
        let tenant_id = Uuid::now_v7();
        let client = make_client_with_mfa_policy(tenant_id, required_mfa_policy());
        let ctx = setup_with_client(tenant_id, true, client);
        let totp_id = seed_totp(&ctx);
        seed_webauthn(&ctx);

        let mut command = make_command(&ctx.email);
        command.mfa_methods = Some(vec!["webauthn".to_string(), "totp".to_string()]);
        command.mfa_default_method = Some("totp".to_string());

        let response = ctx.use_case.execute(command).await.unwrap();

        assert_eq!(response.mfa_type.as_deref(), Some(MfaMethod::Totp.as_str()));
        assert_eq!(response.mfa_id, Some(totp_id.to_string()));
    }

    #[tokio::test]
    async fn uses_configured_mfa_order_when_no_default_or_user_preference() {
        let tenant_id = Uuid::now_v7();
        let client = make_client_with_mfa_policy(tenant_id, required_mfa_policy());
        let ctx = setup_with_client(tenant_id, true, client);
        let totp_id = seed_totp(&ctx);
        seed_webauthn(&ctx);

        let mut command = make_command(&ctx.email);
        command.mfa_methods = Some(vec!["totp".to_string(), "webauthn".to_string()]);

        let response = ctx.use_case.execute(command).await.unwrap();

        assert_eq!(response.mfa_type.as_deref(), Some(MfaMethod::Totp.as_str()));
        assert_eq!(response.mfa_id, Some(totp_id.to_string()));
    }

    #[tokio::test]
    async fn ignores_invalid_default_mfa_hint() {
        let tenant_id = Uuid::now_v7();
        let client = make_client_with_mfa_policy(tenant_id, required_mfa_policy());
        let ctx = setup_with_client(tenant_id, true, client);
        seed_totp(&ctx);
        seed_webauthn(&ctx);

        let mut command = make_command(&ctx.email);
        command.mfa_default_method = Some("sms".to_string());

        let response = ctx.use_case.execute(command).await.unwrap();

        assert_eq!(
            response.mfa_type.as_deref(),
            Some(MfaMethod::Webauthn.as_str())
        );
        assert!(response.web_authn_options.is_some());
    }

    #[tokio::test]
    async fn mfa_hints_cannot_enable_method_disabled_by_client_policy() {
        let tenant_id = Uuid::now_v7();
        let client = make_client_with_mfa_policy(
            tenant_id,
            MfaPolicy {
                required: true,
                email_enabled: true,
                totp_enabled: false,
                webauthn_enabled: false,
            },
        );
        let ctx = setup_with_client(tenant_id, true, client);
        seed_totp(&ctx);

        let mut command = make_command(&ctx.email);
        command.mfa_methods = Some(vec!["totp".to_string()]);
        command.mfa_default_method = Some("totp".to_string());

        let response = ctx.use_case.execute(command).await.unwrap();

        assert_eq!(
            response.mfa_type.as_deref(),
            Some(MfaMethod::Email.as_str())
        );
        assert!(response.mfa_id.is_none());
    }

    #[tokio::test]
    async fn user_preferred_mfa_method_wins_over_hints() {
        let tenant_id = Uuid::now_v7();
        let client = make_client_with_mfa_policy(tenant_id, required_mfa_policy());
        let ctx = setup_with_client(tenant_id, true, client);
        seed_totp(&ctx);
        seed_webauthn(&ctx);
        ctx.users
            .preferred_2fa_methods
            .lock()
            .unwrap()
            .insert(ctx.user_id, MfaMethod::Webauthn.as_str().to_string());

        let mut command = make_command(&ctx.email);
        command.mfa_methods = Some(vec!["totp".to_string(), "webauthn".to_string()]);
        command.mfa_default_method = Some("totp".to_string());

        let response = ctx.use_case.execute(command).await.unwrap();

        assert_eq!(
            response.mfa_type.as_deref(),
            Some(MfaMethod::Webauthn.as_str())
        );
        assert!(response.web_authn_options.is_some());
    }

    // ── Risk Engine integration ──────────────────────────────────────────────

    #[tokio::test]
    async fn pre_auth_ip_block_denies_login_generically() {
        use crate::testing::fake_risk_engine::FakeRiskEngine;
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let ctx = setup_with_client_and_risk(
            tenant_id,
            true,
            client,
            Arc::new(FakeRiskEngine::blocking_pre_auth()),
        );

        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn block_temporarily_decision_denies_valid_login() {
        use crate::domain::entities::login_event::RiskDecision;
        use crate::testing::fake_risk_engine::FakeRiskEngine;
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let ctx = setup_with_client_and_risk(
            tenant_id,
            true,
            client,
            Arc::new(FakeRiskEngine::with_decision(
                RiskDecision::BlockTemporarily,
            )),
        );

        // Credentials are valid, but the risk decision blocks — still a generic error.
        let err = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn allow_with_mfa_forces_step_up_on_non_mfa_client() {
        use crate::domain::entities::login_event::RiskDecision;
        use crate::testing::fake_risk_engine::FakeRiskEngine;
        let tenant_id = Uuid::now_v7();
        // Client does NOT require MFA, but allows email as a factor.
        let client = make_client_with_mfa_policy(
            tenant_id,
            MfaPolicy {
                required: false,
                email_enabled: true,
                totp_enabled: false,
                webauthn_enabled: false,
            },
        );
        let ctx = setup_with_client_and_risk(
            tenant_id,
            true,
            client,
            Arc::new(FakeRiskEngine::with_decision(RiskDecision::AllowWithMfa)),
        );

        let response = ctx
            .use_case
            .execute(make_command(&ctx.email))
            .await
            .unwrap();

        // Risk raised the login to ALLOW_WITH_MFA → email OTP step-up engaged.
        assert_eq!(
            response.mfa_type.as_deref(),
            Some(MfaMethod::Email.as_str())
        );
        assert!(
            response
                .available_mfa_methods
                .contains(&MfaMethod::Email.as_str().to_string())
        );
    }
}
