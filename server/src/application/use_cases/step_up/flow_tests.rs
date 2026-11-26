// Step-up end to end: `start` picks the proof, `confirm` checks it, for the three
// kinds of account (a password, a passkey without a password, neither).
use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::use_cases::step_up::{
        confirm::{
            command::{Command as ConfirmCommand, Proof, Requirement},
            use_case::{PASSWORD_REQUIRED, STEP_UP_REQUIRED, UseCase as Confirm, WRONG_PROOF},
        },
        start::{
            command::Command as StartCommand,
            response::Response,
            use_case::{MAX_CODES_PER_WINDOW, UseCase as Start},
        },
    },
    domain::{
        entities::{
            email_provider::{EmailProvider, SmtpEncryption},
            mfa_challenge::MfaChallenge,
            mfa_enrollment::MfaEnrollment,
            tenant::Tenant,
            user::User,
            user_credentials::UserCredentials,
        },
        errors::DomainError,
    },
    testing::{
        fake_email_renderer::var_from_text,
        fake_webauthn_service::{FAKE_CREDENTIAL_ID, FakeWebauthnService},
        in_memory_repositories::{
            in_memory_email_provider_repository::InMemoryEmailProviderRepository,
            in_memory_mfa_challenge_repository::InMemoryMfaChallengeRepository,
            in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        password_hasher::test_password_hasher,
        recording_email_sender::RecordingEmailSender,
        templated_email_support::recording_templated_email_sender,
        user_factory::{TEST_PASSWORD, TEST_PASSWORD_SECRET, generate_users},
    },
};

#[derive(Clone, Copy, PartialEq)]
enum Account {
    Password,
    Passkey,
    Social,
}

struct Ctx {
    start: Start,
    confirm: Confirm,
    tenant_id: Uuid,
    user_id: Uuid,
    users: Arc<InMemoryUserRepository>,
    challenges: Arc<InMemoryMfaChallengeRepository>,
    enrollments: Arc<InMemoryMfaEnrollmentRepository>,
    email: Arc<RecordingEmailSender>,
}

fn setup(account: Account) -> Ctx {
    setup_with(account, true, true)
}

fn setup_with(account: Account, email_verified: bool, with_provider: bool) -> Ctx {
    let tenant = Tenant::new(
        "Acme".to_string(),
        None,
        TEST_PASSWORD_SECRET.to_string(),
        true,
        true,
        true,
        Uuid::now_v7(),
    );
    let tenant_id = tenant.id();
    let tenants = Arc::new(InMemoryTenantRepository::new());
    tenants
        .memory
        .data
        .lock()
        .unwrap()
        .insert(tenant_id, tenant);

    let mut user = match account {
        Account::Password => generate_users(tenant_id, 1).remove(0),
        _ => User::new(tenant_id, "nopassword@example.com".to_string(), None, None),
    };
    if email_verified {
        user.verify_email();
    }
    let user_id = user.id();
    let users = Arc::new(InMemoryUserRepository::new());
    users.memory.data.lock().unwrap().insert(user_id, user);

    let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
    if account == Account::Passkey {
        let passkey = MfaEnrollment::new_webauthn(
            tenant_id,
            user_id,
            json!({ "fake_passkey": true, "cred_id": FAKE_CREDENTIAL_ID, "counter": 0 }),
            FAKE_CREDENTIAL_ID.to_string(),
            Some("Laptop".to_string()),
        );
        enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(passkey.id(), passkey);
    }

    let providers = Arc::new(InMemoryEmailProviderRepository::new());
    if with_provider {
        let provider = EmailProvider::new(
            tenant_id,
            "Main".to_string(),
            "smtp.example.com".to_string(),
            587,
            "mailer".to_string(),
            "v1:sealed".to_string(),
            "no-reply@example.com".to_string(),
            None,
            SmtpEncryption::StartTls,
            true,
        );
        providers
            .memory
            .data
            .lock()
            .unwrap()
            .insert(provider.id(), provider);
    }

    let challenges = Arc::new(InMemoryMfaChallengeRepository::new());
    let factory = Arc::new(
        InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_mfa_enrollment_repository(enrollments.clone())
            .with_mfa_challenge_repository(challenges.clone())
            .with_email_provider_repository(providers),
    );
    let (templated_email, email) = recording_templated_email_sender();
    let webauthn = Arc::new(FakeWebauthnService::new());

    Ctx {
        start: Start::new(factory.clone(), webauthn.clone(), templated_email, true),
        confirm: Confirm::new(factory, test_password_hasher(), webauthn),
        tenant_id,
        user_id,
        users,
        challenges,
        enrollments,
        email,
    }
}

impl Ctx {
    async fn start(&self) -> Result<Response, DomainError> {
        self.start
            .execute(StartCommand {
                tenant_id: self.tenant_id,
                user_id: self.user_id,
            })
            .await
    }

    async fn confirm(
        &self,
        current_password: Option<&str>,
        proof: Option<Proof>,
        requirement: Requirement,
    ) -> Result<(), DomainError> {
        self.confirm
            .execute(ConfirmCommand {
                tenant_id: self.tenant_id,
                user_id: self.user_id,
                current_password: current_password.map(str::to_string),
                proof,
                requirement,
            })
            .await
    }

    fn challenge(&self, id: Uuid) -> MfaChallenge {
        self.challenges.memory.data.lock().unwrap()[&id].clone()
    }

    fn last_emailed_code(&self) -> String {
        let sent = self.email.sent.lock().unwrap();
        var_from_text(&sent.last().unwrap().text, "code").unwrap()
    }
}

fn passkey_proof(challenge_id: Uuid) -> Proof {
    Proof {
        challenge_id,
        assertion: Some(FakeWebauthnService::valid_client_payload()),
        code: None,
    }
}

fn code_proof(challenge_id: Uuid, code: &str) -> Proof {
    Proof {
        challenge_id,
        assertion: None,
        code: Some(code.to_string()),
    }
}

fn step_up_error(result: Result<(), DomainError>) -> String {
    match result {
        Err(DomainError::StepUpRequired(message)) => message,
        other => panic!("expected StepUpRequired, got {other:?}"),
    }
}

// ── An account with a password ───────────────────────────────────────────────

#[tokio::test]
async fn a_password_account_confirms_with_its_password() {
    let ctx = setup(Account::Password);
    assert!(matches!(ctx.start().await.unwrap(), Response::Password));

    assert_eq!(
        step_up_error(ctx.confirm(None, None, Requirement::Always).await),
        PASSWORD_REQUIRED
    );
    assert!(
        ctx.confirm(Some("not-the-password"), None, Requirement::Always)
            .await
            .is_err()
    );
    ctx.confirm(Some(TEST_PASSWORD), None, Requirement::Always)
        .await
        .unwrap();
    // For it a new passkey is only a second factor: nothing to confirm.
    ctx.confirm(None, None, Requirement::WithoutPassword)
        .await
        .unwrap();
    assert!(ctx.email.sent.lock().unwrap().is_empty());
}

// An admin reset's temporary password matches the stored hash, but it is not the
// user's own: it confirms nothing.
#[tokio::test]
async fn a_password_awaiting_a_forced_change_confirms_nothing() {
    let ctx = setup(Account::Password);
    {
        let mut users = ctx.users.memory.data.lock().unwrap();
        let user = users.get_mut(&ctx.user_id).unwrap();
        let hash = user.credentials().unwrap().password_hash().to_string();
        user.set_credentials(UserCredentials::temporary(
            ctx.user_id,
            hash,
            "argon2".to_string(),
            Utc::now() + Duration::hours(24),
        ));
    }
    assert!(
        ctx.confirm(Some(TEST_PASSWORD), None, Requirement::Always)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_user_of_another_tenant_is_refused() {
    let ctx = setup(Account::Password);
    let err = ctx
        .confirm
        .execute(ConfirmCommand {
            tenant_id: Uuid::now_v7(),
            user_id: ctx.user_id,
            current_password: Some(TEST_PASSWORD.to_string()),
            proof: None,
            requirement: Requirement::Always,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::AccessDenied(_)));
}

// ── No password, a passkey ───────────────────────────────────────────────────

#[tokio::test]
async fn an_account_without_a_password_needs_a_fresh_passkey_assertion() {
    let ctx = setup(Account::Passkey);

    // The token alone, or a password field, is no longer enough.
    assert_eq!(
        step_up_error(
            ctx.confirm(Some("anything"), None, Requirement::Always)
                .await
        ),
        STEP_UP_REQUIRED
    );
    // Adding a passkey asks for it too: a passkey opens this account on its own.
    assert_eq!(
        step_up_error(ctx.confirm(None, None, Requirement::WithoutPassword).await),
        STEP_UP_REQUIRED
    );

    let Response::Passkey {
        challenge_id,
        options,
    } = ctx.start().await.unwrap()
    else {
        panic!("expected a passkey ceremony");
    };
    assert!(options.get("publicKey").is_some());
    ctx.confirm(None, Some(passkey_proof(challenge_id)), Requirement::Always)
        .await
        .unwrap();

    // Single use, and the signature counter moved forward.
    assert!(ctx.challenge(challenge_id).is_consumed());
    let counter = ctx
        .enrollments
        .memory
        .data
        .lock()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .passkey_json()
        .unwrap()["counter"]
        .clone();
    assert_eq!(counter, json!(1));
    assert_eq!(
        step_up_error(
            ctx.confirm(None, Some(passkey_proof(challenge_id)), Requirement::Always)
                .await
        ),
        WRONG_PROOF
    );
    assert!(ctx.email.sent.lock().unwrap().is_empty());
}

#[tokio::test]
async fn wrong_assertions_are_counted_until_the_ceremony_dies() {
    let ctx = setup(Account::Passkey);
    let Response::Passkey { challenge_id, .. } = ctx.start().await.unwrap() else {
        panic!("expected a passkey ceremony");
    };
    let bad = || Proof {
        challenge_id,
        assertion: Some(json!({ "fake": "forged" })),
        code: None,
    };
    for _ in 0..MfaChallenge::MAX_ATTEMPTS {
        assert!(
            ctx.confirm(None, Some(bad()), Requirement::Always)
                .await
                .is_err()
        );
    }
    assert_eq!(
        ctx.challenge(challenge_id).attempts(),
        MfaChallenge::MAX_ATTEMPTS
    );
    // Dead even for the right passkey; a new ceremony is needed.
    assert!(
        ctx.confirm(None, Some(passkey_proof(challenge_id)), Requirement::Always)
            .await
            .is_err()
    );
}

// A passkey account may not fall back to an emailed code: whoever holds a stolen
// token and the mailbox would otherwise pass without the passkey.
#[tokio::test]
async fn a_passkey_account_cannot_use_a_code_instead() {
    let ctx = setup(Account::Passkey);
    let Response::Passkey { challenge_id, .. } = ctx.start().await.unwrap() else {
        panic!("expected a passkey ceremony");
    };
    assert_eq!(
        step_up_error(
            ctx.confirm(
                None,
                Some(code_proof(challenge_id, "123456")),
                Requirement::Always
            )
            .await
        ),
        WRONG_PROOF
    );
}

#[tokio::test]
async fn another_users_ceremony_proves_nothing() {
    let ctx = setup(Account::Passkey);
    let other = setup(Account::Passkey);
    let Response::Passkey { challenge_id, .. } = other.start().await.unwrap() else {
        panic!("expected a passkey ceremony");
    };
    // Copy the other account's live challenge into this store: still not this user's.
    let foreign = other.challenge(challenge_id);
    ctx.challenges
        .memory
        .data
        .lock()
        .unwrap()
        .insert(challenge_id, foreign);
    assert!(
        ctx.confirm(None, Some(passkey_proof(challenge_id)), Requirement::Always)
            .await
            .is_err()
    );
}

// ── No password, no passkey: an emailed code ─────────────────────────────────

#[tokio::test]
async fn a_social_account_confirms_with_an_emailed_code() {
    let ctx = setup(Account::Social);
    let Response::Email { challenge_id, .. } = ctx.start().await.unwrap() else {
        panic!("expected an emailed code");
    };
    let sent = ctx.email.sent.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "nopassword@example.com");
    let code = ctx.last_emailed_code();

    let wrong = if code == "000000" { "000001" } else { "000000" };
    assert!(
        ctx.confirm(
            None,
            Some(code_proof(challenge_id, wrong)),
            Requirement::Always
        )
        .await
        .is_err()
    );
    assert_eq!(ctx.challenge(challenge_id).attempts(), 1);

    ctx.confirm(
        None,
        Some(code_proof(challenge_id, &code)),
        Requirement::Always,
    )
    .await
    .unwrap();
    assert!(ctx.challenge(challenge_id).is_consumed());
    assert!(
        ctx.confirm(
            None,
            Some(code_proof(challenge_id, &code)),
            Requirement::Always
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn a_new_code_replaces_the_previous_one_and_their_number_is_bounded() {
    let ctx = setup(Account::Social);
    let Response::Email {
        challenge_id: first,
        ..
    } = ctx.start().await.unwrap()
    else {
        panic!("expected an emailed code");
    };
    let first_code = ctx.last_emailed_code();
    for _ in 1..MAX_CODES_PER_WINDOW {
        ctx.start().await.unwrap();
    }
    assert!(
        ctx.confirm(
            None,
            Some(code_proof(first, &first_code)),
            Requirement::Always
        )
        .await
        .is_err()
    );
    let err = ctx.start().await.unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
    assert_eq!(
        ctx.email.sent.lock().unwrap().len() as i64,
        MAX_CODES_PER_WINDOW
    );
}

#[tokio::test]
async fn no_code_goes_to_an_unverified_address_or_without_a_provider() {
    let unverified = setup_with(Account::Social, false, true);
    assert!(matches!(
        unverified.start().await.unwrap_err(),
        DomainError::Validation(_)
    ));
    let no_provider = setup_with(Account::Social, true, false);
    assert!(matches!(
        no_provider.start().await.unwrap_err(),
        DomainError::Validation(_)
    ));
    assert!(unverified.email.sent.lock().unwrap().is_empty());
    assert!(no_provider.email.sent.lock().unwrap().is_empty());
}
