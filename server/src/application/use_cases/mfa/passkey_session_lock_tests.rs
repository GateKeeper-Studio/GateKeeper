// A passkey (passwordless) session binds an account that has proven nothing: only a
// WebAuthn assertion may open it. These tests pin that every login-time MFA use case
// enforces the lock — a mailbox or a TOTP code alone must never become a primary
// factor — and that the same use cases keep working on a password session.
//
// A magic-link session has the opposite lock: the mailbox is already its primary
// factor, so an email code may not be its second one (it would prove the same mailbox
// twice), while TOTP and passkeys may.

use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::use_cases::mfa::{
        send_email_otp, start_webauthn_login, verify_email_otp, verify_totp,
    },
    domain::{
        entities::{
            auth_session::AuthSession,
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_enrollment::MfaEnrollment,
            mfa_method::MfaMethod,
            oauth_client::{ClientType, MfaPolicy, OAuthClient},
            user::User,
        },
        errors::DomainError,
    },
    testing::{
        fake_webauthn_service::{FAKE_CREDENTIAL_ID, FakeWebauthnService},
        in_memory_repositories::{
            in_memory_auth_session_repository::InMemoryAuthSessionRepository,
            in_memory_mfa_challenge_repository::InMemoryMfaChallengeRepository,
            in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
            in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        recording_email_sender::RecordingEmailSender,
        stub_secret_cipher::StubSecretCipher,
        templated_email_support::recording_templated_email_sender,
    },
    utils::{oauth_crypto::sha256_hex, totp},
};

const RAW_SESSION_CODE: &str = "raw-session-code-lock";
const PUBLIC_CLIENT_ID: &str = "01890000-0000-7000-8000-000000000abc";
const SECRET_BASE32: &str = "JBSWY3DPEHPK3PXP";
const EMAIL_CODE: &str = "123456";

#[derive(Clone, Copy, PartialEq)]
enum Opened {
    ByPassword,
    ByPasskeyOnly,
    ByMagicLink,
}

struct Ctx {
    factory: Arc<InMemoryUnitOfWorkFactory>,
    sessions: Arc<InMemoryAuthSessionRepository>,
    challenges: Arc<InMemoryMfaChallengeRepository>,
    emails: Arc<RecordingEmailSender>,
    templated_email:
        Arc<crate::application::services::templated_email_sender::TemplatedEmailSender>,
    tenant_id: Uuid,
    user_id: Uuid,
    session_id: Uuid,
}

fn new_session(tenant_id: Uuid, client: &OAuthClient, token: &str) -> AuthSession {
    let expires_at = Utc::now() + Duration::minutes(10);
    AuthSession::new(
        tenant_id,
        sha256_hex(token),
        client.id(),
        "https://app.example.com/callback".to_string(),
        vec!["openid".to_string()],
        "xyz".to_string(),
        "challenge".to_string(),
        "S256".to_string(),
        None,
        expires_at,
        expires_at + Duration::minutes(20),
    )
}

// A verified account (no password, so a passkey session is legitimate for it) with
// TOTP and a passkey enrolled, and one session parked at the MFA step.
fn setup(opened: Opened) -> Ctx {
    let tenant_id = Uuid::now_v7();
    let client = OAuthClient::new(
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
    );

    let mut user = User::new(tenant_id, "lock.user@example.com".to_string(), None, None);
    user.verify_email();
    let user_id = user.id();
    let users = Arc::new(InMemoryUserRepository::new());
    users.memory.data.lock().unwrap().insert(user_id, user);

    let mut session = new_session(tenant_id, &client, RAW_SESSION_CODE);
    match opened {
        Opened::ByPassword => {
            session.authenticate(user_id, Utc::now());
            session.require_mfa();
        }
        Opened::ByPasskeyOnly => session.identify_for_passkey(user_id),
        Opened::ByMagicLink => {
            session.await_magic_link();
            session.authenticate(user_id, Utc::now());
            session.require_mfa();
        }
    }
    let session_id = session.id();
    let sessions = Arc::new(InMemoryAuthSessionRepository::new());
    sessions
        .memory
        .data
        .lock()
        .unwrap()
        .insert(session_id, session);

    let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
    let mut totp_enrollment =
        MfaEnrollment::new_totp(tenant_id, user_id, format!("enc:{SECRET_BASE32}"));
    totp_enrollment.confirm(Utc::now());
    let passkey = MfaEnrollment::new_webauthn(
        tenant_id,
        user_id,
        json!({ "fake_passkey": true, "cred_id": FAKE_CREDENTIAL_ID, "counter": 0 }),
        FAKE_CREDENTIAL_ID.to_string(),
        None,
    );
    {
        let mut data = enrollments.memory.data.lock().unwrap();
        data.insert(totp_enrollment.id(), totp_enrollment);
        data.insert(passkey.id(), passkey);
    }

    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    clients
        .memory
        .data
        .lock()
        .unwrap()
        .insert(client.id(), client);

    let challenges = Arc::new(InMemoryMfaChallengeRepository::new());
    let factory = Arc::new(
        InMemoryUnitOfWorkFactory::new(users, Arc::new(InMemoryTenantRepository::new()))
            .with_oauth_client_repository(clients)
            .with_auth_session_repository(sessions.clone())
            .with_mfa_enrollment_repository(enrollments)
            .with_mfa_challenge_repository(challenges.clone()),
    );
    let (templated_email, emails) = recording_templated_email_sender();

    Ctx {
        factory,
        sessions,
        challenges,
        emails,
        templated_email,
        tenant_id,
        user_id,
        session_id,
    }
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

fn seed_email_challenge(ctx: &Ctx) {
    let challenge = MfaChallenge::new_email_otp(
        ctx.tenant_id,
        ctx.user_id,
        Some(ctx.session_id),
        MfaChallengePurpose::Login,
        sha256_hex(EMAIL_CODE),
        Utc::now() + Duration::minutes(10),
    );
    ctx.challenges
        .memory
        .data
        .lock()
        .unwrap()
        .insert(challenge.id(), challenge);
}

fn assert_refused(result: Result<impl std::fmt::Debug, DomainError>) {
    let err = result.unwrap_err();
    assert!(
        matches!(err, DomainError::AuthenticationError(_)),
        "expected the generic refusal, got {err:?}"
    );
}

// ── send_email_otp ───────────────────────────────────────────────────────────

async fn send_email_otp(ctx: &Ctx) -> Result<(), DomainError> {
    send_email_otp::use_case::UseCase::new(ctx.factory.clone(), ctx.templated_email.clone())
        .execute(send_email_otp::command::Command {
            client_id: PUBLIC_CLIENT_ID.to_string(),
            session_code: RAW_SESSION_CODE.to_string(),
        })
        .await
}

#[tokio::test]
async fn send_email_otp_sends_a_code_for_a_password_session() {
    let ctx = setup(Opened::ByPassword);

    send_email_otp(&ctx).await.unwrap();

    assert_eq!(
        ctx.emails.recipients(),
        vec!["lock.user@example.com".to_string()]
    );
}

#[tokio::test]
async fn send_email_otp_refuses_a_passkey_session() {
    let ctx = setup(Opened::ByPasskeyOnly);

    assert_refused(send_email_otp(&ctx).await);

    assert!(ctx.emails.recipients().is_empty());
    assert!(ctx.challenges.memory.data.lock().unwrap().is_empty());
}

#[tokio::test]
async fn send_email_otp_refuses_a_magic_link_session() {
    let ctx = setup(Opened::ByMagicLink);

    assert_refused(send_email_otp(&ctx).await);

    assert!(ctx.emails.recipients().is_empty());
    assert!(ctx.challenges.memory.data.lock().unwrap().is_empty());
}

// ── verify_email_otp ─────────────────────────────────────────────────────────

async fn verify_email_otp(ctx: &Ctx) -> Result<verify_email_otp::response::Response, DomainError> {
    verify_email_otp::use_case::UseCase::new(ctx.factory.clone())
        .execute(verify_email_otp::command::Command {
            client_id: PUBLIC_CLIENT_ID.to_string(),
            session_code: RAW_SESSION_CODE.to_string(),
            code: EMAIL_CODE.to_string(),
        })
        .await
}

#[tokio::test]
async fn verify_email_otp_clears_a_password_session() {
    let ctx = setup(Opened::ByPassword);
    seed_email_challenge(&ctx);

    verify_email_otp(&ctx).await.unwrap();

    assert!(!session(&ctx).is_mfa_pending());
}

#[tokio::test]
async fn verify_email_otp_refuses_a_passkey_session_even_with_the_right_code() {
    let ctx = setup(Opened::ByPasskeyOnly);
    seed_email_challenge(&ctx);

    assert_refused(verify_email_otp(&ctx).await);

    assert!(session(&ctx).is_mfa_pending());
}

#[tokio::test]
async fn verify_email_otp_refuses_a_magic_link_session_even_with_the_right_code() {
    let ctx = setup(Opened::ByMagicLink);
    seed_email_challenge(&ctx);

    assert_refused(verify_email_otp(&ctx).await);

    assert!(session(&ctx).is_mfa_pending());
}

// ── verify_totp ──────────────────────────────────────────────────────────────

async fn verify_totp(ctx: &Ctx) -> Result<verify_totp::response::Response, DomainError> {
    let secret =
        base32::decode(base32::Alphabet::Rfc4648 { padding: false }, SECRET_BASE32).unwrap();
    let code = totp::generate(&secret, totp::step_for(Utc::now().timestamp()));
    verify_totp::use_case::UseCase::new(ctx.factory.clone(), Arc::new(StubSecretCipher))
        .execute(verify_totp::command::Command {
            client_id: PUBLIC_CLIENT_ID.to_string(),
            session_code: RAW_SESSION_CODE.to_string(),
            mfa_id: None,
            code,
        })
        .await
}

#[tokio::test]
async fn verify_totp_refuses_a_passkey_session_even_with_the_right_code() {
    let ctx = setup(Opened::ByPasskeyOnly);

    assert_refused(verify_totp(&ctx).await);

    assert!(session(&ctx).is_mfa_pending());
    // Refused at the gate: not even the attempt-counter row is created.
    assert!(ctx.challenges.memory.data.lock().unwrap().is_empty());
}

#[tokio::test]
async fn verify_totp_still_clears_a_password_session() {
    let ctx = setup(Opened::ByPassword);

    verify_totp(&ctx).await.unwrap();

    assert!(!session(&ctx).is_mfa_pending());
}

#[tokio::test]
async fn verify_totp_clears_a_magic_link_session() {
    let ctx = setup(Opened::ByMagicLink);

    verify_totp(&ctx).await.unwrap();

    assert!(!session(&ctx).is_mfa_pending());
}

// ── start_webauthn_login ─────────────────────────────────────────────────────

async fn start_webauthn(
    ctx: &Ctx,
    session_code: &str,
) -> Result<start_webauthn_login::response::Response, DomainError> {
    start_webauthn_login::use_case::UseCase::new(
        ctx.factory.clone(),
        Arc::new(FakeWebauthnService::new()),
    )
    .execute(start_webauthn_login::command::Command {
        client_id: PUBLIC_CLIENT_ID.to_string(),
        session_code: session_code.to_string(),
    })
    .await
}

#[tokio::test]
async fn start_webauthn_issues_a_session_bound_challenge_for_a_passkey_session() {
    let ctx = setup(Opened::ByPasskeyOnly);

    let response = start_webauthn(&ctx, RAW_SESSION_CODE).await.unwrap();

    let challenges = ctx.challenges.memory.data.lock().unwrap();
    let challenge = challenges
        .values()
        .find(|c| c.id().to_string() == response.mfa_id)
        .expect("challenge stored");
    assert_eq!(challenge.auth_session_id(), Some(ctx.session_id));
    assert_eq!(*challenge.method(), MfaMethod::Webauthn);
    assert_eq!(*challenge.purpose(), MfaChallengePurpose::Login);
}

#[tokio::test]
async fn start_webauthn_replaces_only_this_sessions_ceremony() {
    // Anyone who knows the email can open a passkey session; restarting the
    // ceremony there must not cancel the real user's sign-in in another session.
    let ctx = setup(Opened::ByPasskeyOnly);

    let other_code = "raw-session-code-other";
    let client_id = session(&ctx).pending_client_id();
    let other = {
        let mut s = AuthSession::new(
            ctx.tenant_id,
            sha256_hex(other_code),
            client_id,
            "https://app.example.com/callback".to_string(),
            vec!["openid".to_string()],
            "xyz".to_string(),
            "challenge".to_string(),
            "S256".to_string(),
            None,
            Utc::now() + Duration::minutes(10),
            Utc::now() + Duration::minutes(30),
        );
        s.identify_for_passkey(ctx.user_id);
        s
    };
    ctx.sessions
        .memory
        .data
        .lock()
        .unwrap()
        .insert(other.id(), other);

    let first = start_webauthn(&ctx, RAW_SESSION_CODE).await.unwrap();
    let theirs = start_webauthn(&ctx, other_code).await.unwrap();
    let second = start_webauthn(&ctx, RAW_SESSION_CODE).await.unwrap();

    let challenges = ctx.challenges.memory.data.lock().unwrap();
    let consumed = |id: &str| {
        challenges
            .values()
            .find(|c| c.id().to_string() == id)
            .unwrap()
            .is_consumed()
    };
    assert!(
        consumed(&first.mfa_id),
        "this session's old ceremony is replaced"
    );
    assert!(!consumed(&second.mfa_id));
    assert!(
        !consumed(&theirs.mfa_id),
        "the other session's ceremony survives"
    );
}
