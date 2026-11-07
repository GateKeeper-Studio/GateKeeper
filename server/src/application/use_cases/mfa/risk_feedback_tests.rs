// Weak labels for the statistical risk model (docs/RISK-SCORING.md): a step-up MFA
// the risk engine forced labels its login event by outcome — passed is a weak
// "legitimate", exhausted attempts a weak "fraudulent" — and an admin's label is
// never overwritten by either. MFA a client requires anyway labels nothing.
#![cfg(test)]

use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    application::use_cases::mfa::{verify_email_otp, verify_totp},
    domain::{
        entities::{
            auth_session::AuthSession,
            login_event_label::{LabelSource, LoginEventLabel, RiskLabel},
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_enrollment::MfaEnrollment,
            oauth_client::{ClientType, MfaPolicy, OAuthClient},
            user::User,
        },
        repositories::login_event_label_repository::LoginEventLabelRepository,
    },
    testing::{
        in_memory_repositories::{
            in_memory_auth_session_repository::InMemoryAuthSessionRepository,
            in_memory_login_event_label_repository::InMemoryLoginEventLabelRepository,
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

const SESSION_CODE: &str = "raw-session-code-risk";
const PUBLIC_CLIENT_ID: &str = "01890000-0000-7000-8000-000000000def";
const SECRET_BASE32: &str = "JBSWY3DPEHPK3PXP";
const EMAIL_CODE: &str = "654321";

struct Ctx {
    factory: Arc<InMemoryUnitOfWorkFactory>,
    labels: Arc<InMemoryLoginEventLabelRepository>,
    challenges: Arc<InMemoryMfaChallengeRepository>,
    tenant_id: Uuid,
    user_id: Uuid,
    session_id: Uuid,
    // The login event that forced the step-up (None: client-required MFA).
    risk_event: Option<Uuid>,
}

fn setup(risk_forced: bool) -> Ctx {
    let tenant_id = Uuid::now_v7();
    let client = OAuthClient::new(
        tenant_id,
        Uuid::parse_str(PUBLIC_CLIENT_ID).unwrap(),
        None,
        ClientType::Public,
        "Risk App".to_string(),
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
    let mut user = User::new(tenant_id, "risk.user@example.com".to_string(), None, None);
    user.verify_email();
    let user_id = user.id();
    let users = Arc::new(InMemoryUserRepository::new());
    users.memory.data.lock().unwrap().insert(user_id, user);

    let expires_at = Utc::now() + Duration::minutes(10);
    let mut session = AuthSession::new(
        tenant_id,
        sha256_hex(SESSION_CODE),
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
    session.authenticate(user_id, Utc::now());
    session.require_mfa();
    let risk_event = risk_forced.then(Uuid::now_v7);
    session.set_risk_login_event_id(risk_event);
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
    enrollments
        .memory
        .data
        .lock()
        .unwrap()
        .insert(totp_enrollment.id(), totp_enrollment);

    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    clients
        .memory
        .data
        .lock()
        .unwrap()
        .insert(client.id(), client);

    let challenges = Arc::new(InMemoryMfaChallengeRepository::new());
    let labels = Arc::new(InMemoryLoginEventLabelRepository::new());
    let factory = Arc::new(
        InMemoryUnitOfWorkFactory::new(users, Arc::new(InMemoryTenantRepository::new()))
            .with_oauth_client_repository(clients)
            .with_auth_session_repository(sessions)
            .with_mfa_enrollment_repository(enrollments)
            .with_mfa_challenge_repository(challenges.clone())
            .with_login_event_label_repository(labels.clone()),
    );

    Ctx {
        factory,
        labels,
        challenges,
        tenant_id,
        user_id,
        session_id,
        risk_event,
    }
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

async fn verify_email(ctx: &Ctx, code: &str) -> bool {
    verify_email_otp::use_case::UseCase::new(ctx.factory.clone())
        .execute(verify_email_otp::command::Command {
            client_id: PUBLIC_CLIENT_ID.to_string(),
            session_code: SESSION_CODE.to_string(),
            code: code.to_string(),
        })
        .await
        .is_ok()
}

async fn verify_totp(ctx: &Ctx, right_code: bool) -> bool {
    let secret =
        base32::decode(base32::Alphabet::Rfc4648 { padding: false }, SECRET_BASE32).unwrap();
    let code = if right_code {
        totp::generate(&secret, totp::step_for(Utc::now().timestamp()))
    } else {
        "000000".to_string()
    };
    verify_totp::use_case::UseCase::new(ctx.factory.clone(), Arc::new(StubSecretCipher))
        .execute(verify_totp::command::Command {
            client_id: PUBLIC_CLIENT_ID.to_string(),
            session_code: SESSION_CODE.to_string(),
            mfa_id: None,
            code,
        })
        .await
        .is_ok()
}

fn only_label(ctx: &Ctx) -> Option<(RiskLabel, LabelSource)> {
    let labels = ctx.labels.all();
    assert!(labels.len() <= 1);
    let label = labels.first()?;
    assert_eq!(Some(label.login_event_id()), ctx.risk_event);
    Some((label.label(), label.source()))
}

#[tokio::test]
async fn passing_a_risk_forced_email_step_up_labels_the_login_legitimate() {
    let ctx = setup(true);
    seed_email_challenge(&ctx);

    assert!(verify_email(&ctx, EMAIL_CODE).await);

    assert_eq!(
        only_label(&ctx),
        Some((RiskLabel::Legitimate, LabelSource::MfaPassed))
    );
}

#[tokio::test]
async fn exhausting_a_risk_forced_email_step_up_labels_the_login_fraudulent() {
    let ctx = setup(true);
    seed_email_challenge(&ctx);

    for attempt in 1..=MfaChallenge::MAX_ATTEMPTS {
        assert!(!verify_email(&ctx, "000000").await);
        if attempt < MfaChallenge::MAX_ATTEMPTS {
            assert_eq!(only_label(&ctx), None, "one wrong code is not a verdict");
        }
    }

    assert_eq!(
        only_label(&ctx),
        Some((RiskLabel::Fraudulent, LabelSource::MfaFailed))
    );
}

#[tokio::test]
async fn totp_outcomes_label_the_same_way() {
    let ctx = setup(true);
    assert!(verify_totp(&ctx, true).await);
    assert_eq!(
        only_label(&ctx),
        Some((RiskLabel::Legitimate, LabelSource::MfaPassed))
    );

    let ctx = setup(true);
    for _ in 0..MfaChallenge::MAX_ATTEMPTS {
        assert!(!verify_totp(&ctx, false).await);
    }
    assert_eq!(
        only_label(&ctx),
        Some((RiskLabel::Fraudulent, LabelSource::MfaFailed))
    );
}

#[tokio::test]
async fn mfa_the_client_requires_anyway_labels_nothing() {
    let ctx = setup(false);
    seed_email_challenge(&ctx);

    assert!(verify_email(&ctx, EMAIL_CODE).await);

    assert!(ctx.labels.all().is_empty());
}

#[tokio::test]
async fn a_weak_label_never_overwrites_an_admins() {
    let ctx = setup(true);
    seed_email_challenge(&ctx);
    let event = ctx.risk_event.unwrap();
    ctx.labels
        .upsert(&LoginEventLabel::by_admin(
            ctx.tenant_id,
            event,
            RiskLabel::Fraudulent,
            None,
            Some("confirmed takeover".to_string()),
        ))
        .await
        .unwrap();

    assert!(verify_email(&ctx, EMAIL_CODE).await);

    assert_eq!(
        only_label(&ctx),
        Some((RiskLabel::Fraudulent, LabelSource::Admin))
    );
}
