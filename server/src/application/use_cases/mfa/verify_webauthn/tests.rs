use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::use_cases::mfa::verify_webauthn::{command::Command, use_case::UseCase},
    domain::{
        entities::{
            audit_action::{USER_LOGIN_FAILURE, USER_LOGIN_SUCCESS},
            audit_event::Outcome,
            auth_session::AuthSession,
            login_event::RiskDecision,
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_enrollment::MfaEnrollment,
            oauth_client::{ClientType, MfaPolicy, OAuthClient},
            user::User,
        },
        errors::DomainError,
    },
    testing::{
        fake_risk_engine::FakeRiskEngine,
        fake_webauthn_service::{FAKE_CREDENTIAL_ID, FakeWebauthnService},
        in_memory_repositories::{
            in_memory_audit_event_repository::InMemoryAuditEventRepository,
            in_memory_auth_session_repository::InMemoryAuthSessionRepository,
            in_memory_mfa_challenge_repository::InMemoryMfaChallengeRepository,
            in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
            in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        user_factory::generate_user,
    },
    utils::oauth_crypto::sha256_hex,
};

const RAW_SESSION_CODE: &str = "raw-session-code-webauthn";
const PUBLIC_CLIENT_ID: &str = "01890000-0000-7000-8000-000000000abc";

// Which factor opened the session under test.
#[derive(Clone, Copy)]
enum Opened {
    // Password verified at login; WebAuthn is the second factor.
    ByPassword,
    // Passwordless login: WebAuthn is the primary factor.
    ByPasskeyOnly,
}

struct Ctx {
    use_case: UseCase,
    sessions: Arc<InMemoryAuthSessionRepository>,
    challenges: Arc<InMemoryMfaChallengeRepository>,
    enrollments: Arc<InMemoryMfaEnrollmentRepository>,
    audit: Arc<InMemoryAuditEventRepository>,
    risk: Arc<FakeRiskEngine>,
    session_id: Uuid,
    challenge_id: Uuid,
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
        MfaPolicy::default(),
    )
}

fn passwordless_user(tenant_id: Uuid) -> User {
    let mut user = User::new(
        tenant_id,
        "passkey.user@example.com".to_string(),
        None,
        None,
    );
    user.verify_email();
    user
}

// A session parked at the WebAuthn step with one live Login challenge, and the
// user's single confirmed passkey. `user` is the account the session is bound to.
fn setup_with(opened: Opened, user: User, risk: FakeRiskEngine) -> Ctx {
    let tenant_id = user.tenant_id();
    let user_id = user.id();
    let client = make_client(tenant_id);

    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    clients
        .memory
        .data
        .lock()
        .unwrap()
        .insert(client.id(), client.clone());

    let users = Arc::new(InMemoryUserRepository::new());
    users.memory.data.lock().unwrap().insert(user_id, user);

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
        expires_at + Duration::minutes(20),
    );
    match opened {
        Opened::ByPassword => {
            session.authenticate(user_id, Utc::now());
            session.require_mfa();
        }
        Opened::ByPasskeyOnly => session.identify_for_passkey(user_id),
    }
    let session_id = session.id();
    let sessions = Arc::new(InMemoryAuthSessionRepository::new());
    sessions
        .memory
        .data
        .lock()
        .unwrap()
        .insert(session_id, session);

    let enrollment = MfaEnrollment::new_webauthn(
        tenant_id,
        user_id,
        json!({ "fake_passkey": true, "cred_id": FAKE_CREDENTIAL_ID, "counter": 0 }),
        FAKE_CREDENTIAL_ID.to_string(),
        Some("Laptop".to_string()),
    );
    let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
    enrollments
        .memory
        .data
        .lock()
        .unwrap()
        .insert(enrollment.id(), enrollment);

    let challenge = MfaChallenge::new_webauthn(
        tenant_id,
        user_id,
        Some(session_id),
        MfaChallengePurpose::Login,
        json!({ "ceremony": "authentication" }),
        Utc::now() + Duration::minutes(5),
    );
    let challenge_id = challenge.id();
    let challenges = Arc::new(InMemoryMfaChallengeRepository::new());
    challenges
        .memory
        .data
        .lock()
        .unwrap()
        .insert(challenge_id, challenge);

    let audit = Arc::new(InMemoryAuditEventRepository::new());
    let factory = InMemoryUnitOfWorkFactory::new(users, Arc::new(InMemoryTenantRepository::new()))
        .with_oauth_client_repository(clients)
        .with_auth_session_repository(sessions.clone())
        .with_mfa_enrollment_repository(enrollments.clone())
        .with_mfa_challenge_repository(challenges.clone())
        .with_audit_event_repository(audit.clone());

    let risk = Arc::new(risk);
    Ctx {
        use_case: UseCase::new(
            Arc::new(factory),
            Arc::new(FakeWebauthnService::new()),
            risk.clone(),
        ),
        sessions,
        challenges,
        enrollments,
        audit,
        risk,
        session_id,
        challenge_id,
        user_id,
    }
}

fn setup(opened: Opened) -> Ctx {
    setup_with(
        opened,
        passwordless_user(Uuid::now_v7()),
        FakeRiskEngine::allow(),
    )
}

fn command(ctx: &Ctx, assertion: serde_json::Value) -> Command {
    Command {
        client_id: PUBLIC_CLIENT_ID.to_string(),
        session_code: RAW_SESSION_CODE.to_string(),
        mfa_id: ctx.challenge_id,
        assertion_data: assertion,
        ip_address: Some("203.0.113.7".to_string()),
        user_agent: Some("test-agent".to_string()),
    }
}

fn valid(ctx: &Ctx) -> Command {
    command(ctx, FakeWebauthnService::valid_client_payload())
}

fn forged(ctx: &Ctx) -> Command {
    command(ctx, json!({ "fake": "forged" }))
}

fn session(ctx: &Ctx) -> AuthSession {
    ctx.sessions
        .memory
        .data
        .lock()
        .unwrap()
        .get(&ctx.session_id)
        .cloned()
        .unwrap()
}

fn challenge(ctx: &Ctx) -> MfaChallenge {
    ctx.challenges
        .memory
        .data
        .lock()
        .unwrap()
        .get(&ctx.challenge_id)
        .cloned()
        .unwrap()
}

fn audit_actions(ctx: &Ctx) -> Vec<(String, Outcome, serde_json::Value)> {
    ctx.audit
        .events
        .lock()
        .unwrap()
        .iter()
        .map(|e| (e.action().to_string(), e.outcome(), e.metadata().clone()))
        .collect()
}

// ── Second factor on a password session ──────────────────────────────────────

#[tokio::test]
async fn verifies_a_password_sessions_second_factor() {
    let ctx = setup(Opened::ByPassword);

    let response = ctx.use_case.execute(valid(&ctx)).await.unwrap();

    assert_eq!(response.session_code, RAW_SESSION_CODE);
    let session = session(&ctx);
    assert!(!session.is_mfa_pending());
    assert_eq!(session.mfa_method(), Some("webauthn"));
    assert!(challenge(&ctx).is_consumed());
    // The advanced signature counter is persisted (clone detection).
    let enrollments = ctx.enrollments.memory.data.lock().unwrap();
    let enrollment = enrollments.values().next().unwrap();
    assert_eq!(enrollment.passkey_json().unwrap()["counter"], 1);
    assert!(enrollment.last_used_at().is_some());
    // A second factor is not a login: no risk event, no login audit here.
    assert!(ctx.risk.recorded().is_empty());
    assert!(audit_actions(&ctx).is_empty());
}

#[tokio::test]
async fn a_failed_assertion_counts_a_persisted_attempt() {
    let ctx = setup(Opened::ByPassword);

    let err = ctx.use_case.execute(forged(&ctx)).await.unwrap_err();

    assert!(matches!(err, DomainError::MfaError(_)));
    assert_eq!(challenge(&ctx).attempts(), 1);
    assert!(session(&ctx).is_mfa_pending());
}

#[tokio::test]
async fn the_challenge_dies_after_max_attempts() {
    let ctx = setup(Opened::ByPassword);

    for _ in 0..MfaChallenge::MAX_ATTEMPTS {
        assert!(ctx.use_case.execute(forged(&ctx)).await.is_err());
    }

    // Even a valid assertion is refused once the challenge is exhausted.
    let err = ctx.use_case.execute(valid(&ctx)).await.unwrap_err();
    assert!(matches!(err, DomainError::AuthenticationError(_)));
    assert!(session(&ctx).is_mfa_pending());
}

#[tokio::test]
async fn refuses_a_challenge_from_another_session() {
    let ctx = setup(Opened::ByPassword);
    let foreign = MfaChallenge::new_webauthn(
        challenge(&ctx).tenant_id(),
        ctx.user_id,
        Some(Uuid::now_v7()),
        MfaChallengePurpose::Login,
        json!({ "ceremony": "authentication" }),
        Utc::now() + Duration::minutes(5),
    );
    let foreign_id = foreign.id();
    ctx.challenges
        .memory
        .data
        .lock()
        .unwrap()
        .insert(foreign_id, foreign);

    let mut cmd = valid(&ctx);
    cmd.mfa_id = foreign_id;
    let err = ctx.use_case.execute(cmd).await.unwrap_err();

    assert!(matches!(err, DomainError::AuthenticationError(_)));
    assert!(session(&ctx).is_mfa_pending());
}

// ── Primary factor on a passkey (passwordless) session ───────────────────────

#[tokio::test]
async fn a_passkey_login_authenticates_scores_and_audits() {
    let ctx = setup(Opened::ByPasskeyOnly);

    ctx.use_case.execute(valid(&ctx)).await.unwrap();

    let session = session(&ctx);
    assert!(!session.is_mfa_pending());
    assert!(session.authenticated_at().is_some());
    assert!(challenge(&ctx).is_consumed());

    let recorded = ctx.risk.recorded();
    assert_eq!(recorded.len(), 1);
    assert!(recorded[0].credentials_valid);
    assert_eq!(recorded[0].user_id, Some(ctx.user_id));

    let audit = audit_actions(&ctx);
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].0, USER_LOGIN_SUCCESS);
    assert_eq!(audit[0].1, Outcome::Success);
    assert_eq!(audit[0].2["method"], "passkey");
}

#[tokio::test]
async fn a_failed_passkey_login_is_scored_and_audited() {
    let ctx = setup(Opened::ByPasskeyOnly);

    assert!(ctx.use_case.execute(forged(&ctx)).await.is_err());

    assert_eq!(challenge(&ctx).attempts(), 1);
    assert!(session(&ctx).is_mfa_pending());
    let recorded = ctx.risk.recorded();
    assert_eq!(recorded.len(), 1);
    assert!(!recorded[0].credentials_valid);
    assert_eq!(
        recorded[0].failure_reason.as_deref(),
        Some("invalid_passkey")
    );
    let audit = audit_actions(&ctx);
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].0, USER_LOGIN_FAILURE);
    assert_eq!(audit[0].2["reason"], "invalid_passkey");
}

#[tokio::test]
async fn a_risk_block_burns_the_challenge_and_the_session() {
    let ctx = setup_with(
        Opened::ByPasskeyOnly,
        passwordless_user(Uuid::now_v7()),
        FakeRiskEngine::with_decision(RiskDecision::BlockTemporarily),
    );

    let err = ctx.use_case.execute(valid(&ctx)).await.unwrap_err();

    assert!(matches!(err, DomainError::AuthenticationError(_)));
    let session = session(&ctx);
    assert!(session.is_expired(Utc::now()));
    assert!(session.is_mfa_pending());
    assert!(challenge(&ctx).is_consumed());
    let audit = audit_actions(&ctx);
    assert_eq!(audit[0].0, USER_LOGIN_FAILURE);
    assert_eq!(audit[0].2["reason"], "risk_blocked");

    // Replaying the same assertion gets nowhere.
    assert!(ctx.use_case.execute(valid(&ctx)).await.is_err());
}

#[tokio::test]
async fn allow_with_mfa_is_satisfied_by_the_passkey() {
    let ctx = setup_with(
        Opened::ByPasskeyOnly,
        passwordless_user(Uuid::now_v7()),
        FakeRiskEngine::with_decision(RiskDecision::AllowWithMfa),
    );

    ctx.use_case.execute(valid(&ctx)).await.unwrap();

    assert!(!session(&ctx).is_mfa_pending());
}

#[tokio::test]
async fn a_passkey_login_is_refused_once_the_account_is_deactivated() {
    let tenant_id = Uuid::now_v7();
    let user = User::reconstruct(
        Uuid::now_v7(),
        "passkey.user@example.com".to_string(),
        tenant_id,
        true,
        None,
        None,
        false,
        Utc::now(),
        None,
    );
    let ctx = setup_with(Opened::ByPasskeyOnly, user, FakeRiskEngine::allow());

    let err = ctx.use_case.execute(valid(&ctx)).await.unwrap_err();

    assert!(matches!(err, DomainError::AuthenticationError(_)));
    assert!(session(&ctx).is_mfa_pending());
    assert!(!challenge(&ctx).is_consumed());
}

#[tokio::test]
async fn a_passkey_login_is_refused_once_the_account_has_a_password() {
    // A reset mid-flow (it does not kill sessions) puts the account back on the
    // password path; the passkey session opened before it must not complete.
    let mut user = generate_user(Uuid::now_v7());
    user.verify_email();
    let ctx = setup_with(Opened::ByPasskeyOnly, user, FakeRiskEngine::allow());

    let err = ctx.use_case.execute(valid(&ctx)).await.unwrap_err();

    assert!(matches!(err, DomainError::AuthenticationError(_)));
    assert!(session(&ctx).is_mfa_pending());
}

#[tokio::test]
async fn a_passkey_login_is_refused_under_a_pre_auth_block() {
    let ctx = setup_with(
        Opened::ByPasskeyOnly,
        passwordless_user(Uuid::now_v7()),
        FakeRiskEngine::blocking_pre_auth(),
    );

    let err = ctx.use_case.execute(valid(&ctx)).await.unwrap_err();

    assert!(matches!(err, DomainError::AuthenticationError(_)));
    assert!(session(&ctx).is_mfa_pending());
    assert!(!challenge(&ctx).is_consumed());
    let recorded = ctx.risk.recorded();
    assert_eq!(recorded.len(), 1);
    assert!(recorded[0].pre_auth_blocked);
}
