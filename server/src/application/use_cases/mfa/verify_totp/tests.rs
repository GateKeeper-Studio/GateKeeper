#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::use_cases::mfa::verify_totp::{command::Command, use_case::UseCase},
        domain::{
            entities::{
                auth_session::AuthSession,
                mfa_challenge::MfaChallenge,
                mfa_enrollment::MfaEnrollment,
                mfa_method::MfaMethod,
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_auth_session_repository::InMemoryAuthSessionRepository,
                in_memory_mfa_challenge_repository::InMemoryMfaChallengeRepository,
                in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            stub_secret_cipher::StubSecretCipher,
        },
        utils::{oauth_crypto::sha256_hex, totp},
    };

    const RAW_SESSION_CODE: &str = "raw-session-code-abc123";
    const SECRET_BASE32: &str = "JBSWY3DPEHPK3PXP";
    const PUBLIC_CLIENT_ID: &str = "01890000-0000-7000-8000-000000000abc";

    struct Ctx {
        use_case: UseCase,
        challenges: Arc<InMemoryMfaChallengeRepository>,
        sessions: Arc<InMemoryAuthSessionRepository>,
        session_id: Uuid,
        user_id: Uuid,
    }

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::new(
            tenant_id,
            Uuid::parse_str(PUBLIC_CLIENT_ID).unwrap(),
            None,
            ClientType::Public,
            "Test App".to_string(),
            vec!["https://app.example.com/callback".to_string()],
            vec![],
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            vec!["en".to_string()],
            "en".to_string(),
            // TOTP must be an allowed factor or the use case rejects before we
            // ever reach the attempt counter.
            MfaPolicy::default(),
        )
    }

    // An authenticated session parked at the MFA step — exactly the state an
    // attacker who has the password (or a stolen session code) would hold.
    fn setup() -> Ctx {
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let client = make_client(tenant_id);

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let expires_at = Utc::now() + Duration::minutes(10);
        let mut session = AuthSession::new(
            tenant_id,
            sha256_hex(RAW_SESSION_CODE),
            client.id(),
            "https://app.example.com/callback".to_string(),
            vec!["openid".to_string()],
            "xyz".to_string(),
            "challenge".to_string(),
            "S256".to_string(),
            None,
            expires_at,
            expires_at + Duration::hours(1),
        );
        session.authenticate(user_id, Utc::now());
        session.require_mfa();
        let session_id = session.id();

        let sessions = Arc::new(InMemoryAuthSessionRepository::new());
        sessions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(session_id, session);

        // Stub cipher stores "enc:<plaintext>".
        let mut enrollment =
            MfaEnrollment::new_totp(tenant_id, user_id, format!("enc:{SECRET_BASE32}"));
        enrollment.confirm(Utc::now());
        let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
        enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);

        let challenges = Arc::new(InMemoryMfaChallengeRepository::new());

        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_oauth_client_repository(clients)
        .with_auth_session_repository(sessions.clone())
        .with_mfa_enrollment_repository(enrollments)
        .with_mfa_challenge_repository(challenges.clone());

        Ctx {
            use_case: UseCase::new(Arc::new(factory), Arc::new(StubSecretCipher)),
            challenges,
            sessions,
            session_id,
            user_id,
        }
    }

    fn command(code: &str) -> Command {
        Command {
            client_id: PUBLIC_CLIENT_ID.to_string(),
            session_code: RAW_SESSION_CODE.to_string(),
            mfa_id: None,
            code: code.to_string(),
        }
    }

    fn current_code() -> String {
        let secret =
            base32::decode(base32::Alphabet::Rfc4648 { padding: false }, SECRET_BASE32).unwrap();
        totp::generate(&secret, totp::step_for(Utc::now().timestamp()))
    }

    #[tokio::test]
    async fn verifies_a_valid_code_and_marks_mfa_satisfied() {
        let ctx = setup();

        ctx.use_case
            .execute(command(&current_code()))
            .await
            .unwrap();

        let sessions = ctx.sessions.memory.data.lock().unwrap();
        assert!(!sessions.get(&ctx.session_id).unwrap().is_mfa_pending());
    }

    // The core of this fix: TOTP has no server-issued code, so before the attempt
    // counter existed a caller holding a valid session code could walk the whole
    // 6-digit space. There is no HTTP rate limit behind this in-memory harness, so
    // this test is what pins the bound.
    #[tokio::test]
    async fn locks_out_after_max_attempts_and_ignores_a_correct_code_afterwards() {
        let ctx = setup();

        for attempt in 1..=MfaChallenge::MAX_ATTEMPTS {
            let err = ctx.use_case.execute(command("000000")).await.unwrap_err();
            assert!(
                matches!(err, DomainError::AuthenticationError(_)),
                "attempt {attempt} should fail generically"
            );
        }

        // Every wrong try was counted, not silently discarded.
        {
            let challenges = ctx.challenges.memory.data.lock().unwrap();
            let challenge = challenges
                .values()
                .find(|c| c.auth_session_id() == Some(ctx.session_id))
                .expect("attempt counter should have been created");
            assert_eq!(challenge.attempts(), MfaChallenge::MAX_ATTEMPTS);
            assert!(challenge.attempts_exhausted());
        }

        // The right code no longer helps — the counter is burned.
        let err = ctx
            .use_case
            .execute(command(&current_code()))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));

        let sessions = ctx.sessions.memory.data.lock().unwrap();
        assert!(
            sessions.get(&ctx.session_id).unwrap().is_mfa_pending(),
            "MFA must still be pending after lockout"
        );
    }

    #[tokio::test]
    async fn a_wrong_code_then_the_right_one_still_succeeds_below_the_cap() {
        let ctx = setup();

        ctx.use_case.execute(command("000000")).await.unwrap_err();
        ctx.use_case
            .execute(command(&current_code()))
            .await
            .unwrap();

        let sessions = ctx.sessions.memory.data.lock().unwrap();
        assert!(!sessions.get(&ctx.session_id).unwrap().is_mfa_pending());
    }

    #[tokio::test]
    async fn the_counter_is_consumed_on_success_so_it_cannot_be_replayed() {
        let ctx = setup();

        ctx.use_case
            .execute(command(&current_code()))
            .await
            .unwrap();

        let challenges = ctx.challenges.memory.data.lock().unwrap();
        let challenge = challenges
            .values()
            .find(|c| c.auth_session_id() == Some(ctx.session_id))
            .expect("attempt counter");
        assert!(challenge.is_consumed());
        assert_eq!(*challenge.method(), MfaMethod::Totp);
        assert_eq!(challenge.user_id(), ctx.user_id);
        // Nothing secret is stored on the counter — there is no code to keep.
        assert!(challenge.code_hash().is_none());
        assert!(challenge.state_json().is_none());
    }
}
