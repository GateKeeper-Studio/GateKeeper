// Registering, listing and removing trusted devices on in-memory repositories, and
// their removal when every session of the user ends.

use std::sync::Arc;

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use chrono::Utc;
use p256::{
    ecdsa::{Signature, SigningKey, signature::Signer},
    pkcs8::EncodePublicKey,
};
use uuid::Uuid;

use crate::{
    application::{
        shared::actor::Actor,
        use_cases::{
            session::revoke_all_user_sessions,
            trusted_device::{list, register, revoke, shared::registration_message},
        },
    },
    domain::{
        entities::{
            audit_action::{USER_TRUSTED_DEVICE_REGISTERED, USER_TRUSTED_DEVICE_REVOKED},
            mfa_enrollment::MfaEnrollment,
            tenant::Tenant,
            trusted_device::MAX_PER_USER,
            user::User,
        },
        errors::DomainError,
    },
    testing::{
        in_memory_repositories::{
            in_memory_audit_event_repository::InMemoryAuditEventRepository,
            in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_trusted_device_repository::InMemoryTrustedDeviceRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        password_hasher::test_password_hasher,
        stub_secret_cipher::StubSecretCipher,
        user_factory::{TEST_PASSWORD, TEST_PASSWORD_SECRET, generate_user},
    },
    utils::totp,
};

// RFC 4648 base32 of "12345678901234567890", the RFC 6238 test secret.
const TOTP_SEED: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

struct Fixture {
    register: register::use_case::UseCase,
    list: list::use_case::UseCase,
    revoke: revoke::use_case::UseCase,
    revoke_all: revoke_all_user_sessions::use_case::UseCase,
    users: Arc<InMemoryUserRepository>,
    devices: Arc<InMemoryTrustedDeviceRepository>,
    enrollments: Arc<InMemoryMfaEnrollmentRepository>,
    audit: Arc<InMemoryAuditEventRepository>,
    tenant_id: Uuid,
    user_id: Uuid,
}

fn build() -> Fixture {
    let tenant_id = Uuid::now_v7();
    let tenants = Arc::new(InMemoryTenantRepository::new());
    let tenant = Tenant::reconstruct(
        tenant_id,
        "Test Tenant".to_string(),
        None,
        TEST_PASSWORD_SECRET.to_string(),
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
    users.memory.data.lock().unwrap().insert(user_id, user);

    let devices = Arc::new(InMemoryTrustedDeviceRepository::new());
    let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
    let audit = Arc::new(InMemoryAuditEventRepository::new());
    let factory = Arc::new(
        InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_trusted_device_repository(devices.clone())
            .with_mfa_enrollment_repository(enrollments.clone())
            .with_audit_event_repository(audit.clone()),
    );
    Fixture {
        register: register::use_case::UseCase::new(
            factory.clone(),
            test_password_hasher(),
            Arc::new(StubSecretCipher),
        ),
        list: list::use_case::UseCase::new(devices.clone()),
        revoke: revoke::use_case::UseCase::new(factory.clone()),
        revoke_all: revoke_all_user_sessions::use_case::UseCase::new(factory),
        users,
        devices,
        enrollments,
        audit,
        tenant_id,
        user_id,
    }
}

fn key(seed: u8) -> SigningKey {
    SigningKey::from_slice(&[seed; 32]).unwrap()
}

// The public key as an Android app sends it: SPKI DER, standard base64.
fn spki(key: &SigningKey) -> String {
    STANDARD.encode(key.verifying_key().to_public_key_der().unwrap().as_bytes())
}

impl Fixture {
    fn command(&self, key: &SigningKey) -> register::command::Command {
        let public_key = spki(key);
        let signature: Signature =
            key.sign(registration_message(self.user_id, &public_key).as_bytes());
        register::command::Command {
            tenant_id: self.tenant_id,
            user_id: self.user_id,
            name: " Pixel 8 ".to_string(),
            public_key,
            signature: URL_SAFE_NO_PAD.encode(signature.to_der().as_bytes()),
            current_password: Some(TEST_PASSWORD.to_string()),
            mfa_code: None,
            ip_address: Some("203.0.113.7".to_string()),
            user_agent: Some("app".to_string()),
        }
    }

    async fn remove(&self, actor: &Actor, device_id: Uuid) -> Result<(), DomainError> {
        self.revoke
            .execute(
                actor,
                revoke::command::Command {
                    tenant_id: self.tenant_id,
                    user_id: self.user_id,
                    device_id,
                    ip_address: None,
                    user_agent: None,
                },
            )
            .await
    }

    fn me(&self) -> Actor {
        Actor::User {
            tenant_id: self.tenant_id,
            user_id: self.user_id,
        }
    }

    fn seed_totp(&self) {
        let mut enrollment =
            MfaEnrollment::new_totp(self.tenant_id, self.user_id, format!("enc:{TOTP_SEED}"));
        enrollment.confirm(Utc::now());
        self.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);
    }

    fn current_code(&self) -> String {
        let secret =
            base32::decode(base32::Alphabet::Rfc4648 { padding: false }, TOTP_SEED).unwrap();
        totp::generate(&secret, totp::step_for(Utc::now().timestamp()))
    }

    fn actions(&self) -> Vec<String> {
        self.audit
            .events
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.action().to_string())
            .collect()
    }
}

#[tokio::test]
async fn an_app_registers_its_key_with_the_password_and_a_proof_of_possession() {
    let fx = build();
    let device = fx.register.execute(fx.command(&key(1))).await.unwrap();
    assert_eq!(device.name, "Pixel 8");
    assert_eq!(device.algorithm, "ES256");
    // No second factor enrolled: the device becomes one.
    assert!(device.counts_as_second_factor);

    let listed = fx
        .list
        .execute(list::query::Query {
            tenant_id: fx.tenant_id,
            user_id: fx.user_id,
        })
        .await
        .unwrap();
    assert_eq!(listed.devices.len(), 1);
    // Stored normalized (an uncompressed SEC1 point), never returned.
    assert_eq!(
        URL_SAFE_NO_PAD
            .decode(fx.devices.all()[0].public_key())
            .unwrap()
            .len(),
        65
    );
    assert!(
        fx.actions()
            .contains(&USER_TRUSTED_DEVICE_REGISTERED.to_string())
    );

    // The same key twice is refused.
    assert!(matches!(
        fx.register.execute(fx.command(&key(1))).await,
        Err(DomainError::Conflict(_))
    ));
}

#[tokio::test]
async fn a_token_alone_cannot_register_a_device() {
    let fx = build();
    let mut missing = fx.command(&key(2));
    missing.current_password = None;
    assert!(matches!(
        fx.register.execute(missing).await,
        Err(DomainError::StepUpRequired(_))
    ));
    let mut wrong = fx.command(&key(2));
    wrong.current_password = Some("not-the-password".to_string());
    assert!(matches!(
        fx.register.execute(wrong).await,
        Err(DomainError::StepUpRequired(_))
    ));
    assert!(fx.devices.all().is_empty());
}

#[tokio::test]
async fn the_signature_must_come_from_the_key_and_name_this_account() {
    let fx = build();
    // Signed by another key.
    let mut foreign = fx.command(&key(3));
    let other: Signature =
        key(4).sign(registration_message(fx.user_id, &foreign.public_key).as_bytes());
    foreign.signature = URL_SAFE_NO_PAD.encode(other.to_der().as_bytes());
    assert!(matches!(
        fx.register.execute(foreign).await,
        Err(DomainError::Validation(_))
    ));
    // Signed for another user.
    let mut replayed = fx.command(&key(3));
    let for_other: Signature =
        key(3).sign(registration_message(Uuid::now_v7(), &replayed.public_key).as_bytes());
    replayed.signature = URL_SAFE_NO_PAD.encode(for_other.to_der().as_bytes());
    assert!(matches!(
        fx.register.execute(replayed).await,
        Err(DomainError::Validation(_))
    ));
    // Not a P-256 key; and no name.
    let mut garbage = fx.command(&key(3));
    garbage.public_key = "bm90IGEga2V5".to_string();
    assert!(matches!(
        fx.register.execute(garbage).await,
        Err(DomainError::Validation(_))
    ));
    let mut unnamed = fx.command(&key(3));
    unnamed.name = "   ".to_string();
    assert!(matches!(
        fx.register.execute(unnamed).await,
        Err(DomainError::Validation(_))
    ));
    assert!(fx.devices.all().is_empty());
}

#[tokio::test]
async fn second_factor_credit_needs_the_users_authenticator_code() {
    let fx = build();
    fx.seed_totp();

    // A user with a second factor who does not present it: the device only replaces
    // the password.
    let plain = fx.register.execute(fx.command(&key(5))).await.unwrap();
    assert!(!plain.counts_as_second_factor);

    // A wrong code is refused.
    let mut wrong = fx.command(&key(6));
    wrong.mfa_code = Some("000000".to_string());
    assert!(matches!(
        fx.register.execute(wrong).await,
        Err(DomainError::MfaError(_))
    ));

    // A current code gives the credit, and the code cannot be used again.
    let code = fx.current_code();
    let mut with_code = fx.command(&key(6));
    with_code.mfa_code = Some(code.clone());
    let credited = fx.register.execute(with_code).await.unwrap();
    assert!(credited.counts_as_second_factor);
    let mut replay = fx.command(&key(7));
    replay.mfa_code = Some(code);
    assert!(matches!(
        fx.register.execute(replay).await,
        Err(DomainError::MfaError(_))
    ));
}

// The authenticator code has six digits: five wrong tries, then no code is accepted
// for a while, the right one included.
#[tokio::test]
async fn wrong_authenticator_codes_run_out() {
    let fx = build();
    fx.seed_totp();
    let current = fx.current_code();
    let wrong_code = if current == "000000" {
        "000001"
    } else {
        "000000"
    };

    for _ in 0..5 {
        let mut wrong = fx.command(&key(6));
        wrong.mfa_code = Some(wrong_code.to_string());
        assert!(matches!(
            fx.register.execute(wrong).await,
            Err(DomainError::MfaError(_))
        ));
    }

    let mut right = fx.command(&key(6));
    right.mfa_code = Some(fx.current_code());
    assert!(matches!(
        fx.register.execute(right).await,
        Err(DomainError::MfaError(message)) if message.contains("too many")
    ));
    assert!(fx.devices.all().is_empty());
}

#[tokio::test]
async fn an_account_has_at_most_ten_devices() {
    let fx = build();
    for seed in 0..MAX_PER_USER as u8 {
        fx.register
            .execute(fx.command(&key(seed + 10)))
            .await
            .unwrap();
    }
    assert!(matches!(
        fx.register.execute(fx.command(&key(99))).await,
        Err(DomainError::Conflict(_))
    ));
}

#[tokio::test]
async fn a_passwordless_account_cannot_register() {
    let fx = build();
    {
        let mut users = fx.users.memory.data.lock().unwrap();
        let user = users.get(&fx.user_id).cloned().unwrap();
        users.insert(
            fx.user_id,
            User::reconstruct(
                user.id(),
                user.email().to_string(),
                user.tenant_id(),
                true,
                user.profile().cloned(),
                None,
                true,
                user.created_at(),
                user.updated_at(),
            ),
        );
    }
    assert!(matches!(
        fx.register.execute(fx.command(&key(8))).await,
        Err(DomainError::AccessDenied(_))
    ));
}

#[tokio::test]
async fn the_user_or_an_admin_removes_a_device_but_nobody_else() {
    let fx = build();
    let device = fx.register.execute(fx.command(&key(20))).await.unwrap();

    let stranger = Actor::User {
        tenant_id: fx.tenant_id,
        user_id: Uuid::now_v7(),
    };
    assert!(matches!(
        fx.remove(&stranger, device.id).await,
        Err(DomainError::AccessDenied(_))
    ));
    fx.remove(&fx.me(), device.id).await.unwrap();
    assert!(fx.devices.all().is_empty());
    assert!(matches!(
        fx.remove(&fx.me(), device.id).await,
        Err(DomainError::NotFound(_))
    ));
    assert!(
        fx.actions()
            .contains(&USER_TRUSTED_DEVICE_REVOKED.to_string())
    );

    // An admin (here the system) removes another one.
    let second = fx.register.execute(fx.command(&key(21))).await.unwrap();
    fx.revoke
        .execute(
            &Actor::System,
            revoke::command::Command {
                tenant_id: fx.tenant_id,
                user_id: fx.user_id,
                device_id: second.id,
                ip_address: None,
                user_agent: None,
            },
        )
        .await
        .unwrap();
    assert!(fx.devices.all().is_empty());
}

#[tokio::test]
async fn signing_out_everywhere_removes_every_trusted_device() {
    let fx = build();
    fx.register.execute(fx.command(&key(30))).await.unwrap();
    fx.register.execute(fx.command(&key(31))).await.unwrap();
    assert_eq!(fx.devices.all().len(), 2);

    fx.revoke_all
        .execute(
            &fx.me(),
            revoke_all_user_sessions::command::Command {
                tenant_id: fx.tenant_id,
                user_id: fx.user_id,
                ip_address: None,
                user_agent: None,
            },
        )
        .await
        .unwrap();
    assert!(fx.devices.all().is_empty());
}
