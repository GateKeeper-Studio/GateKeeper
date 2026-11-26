use std::sync::Arc;

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::use_cases::tenant_user::get_authentication_methods::{
        query::Query, use_case::UseCase,
    },
    domain::{
        entities::{
            external_identity_provider::ExternalProviderKind, mfa_enrollment::MfaEnrollment,
            mfa_method::MfaMethod, social_identity::SocialIdentity, user::User,
            user_credentials::UserCredentials,
        },
        errors::DomainError,
        repositories::user_repository::UserRepository,
    },
    testing::in_memory_repositories::{
        in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
        in_memory_social_identity_repository::InMemorySocialIdentityRepository,
        in_memory_user_repository::InMemoryUserRepository,
    },
};

struct Ctx {
    users: Arc<InMemoryUserRepository>,
    enrollments: Arc<InMemoryMfaEnrollmentRepository>,
    socials: Arc<InMemorySocialIdentityRepository>,
    tenant_id: Uuid,
}

impl Ctx {
    fn new() -> Self {
        Self {
            users: Arc::new(InMemoryUserRepository::new()),
            enrollments: Arc::new(InMemoryMfaEnrollmentRepository::new()),
            socials: Arc::new(InMemorySocialIdentityRepository::new()),
            tenant_id: Uuid::now_v7(),
        }
    }

    fn use_case(&self) -> UseCase {
        UseCase::new(
            self.users.clone(),
            self.enrollments.clone(),
            self.socials.clone(),
        )
    }

    fn add_user(&self, password: bool, verified: bool) -> User {
        let mut user = User::new(self.tenant_id, "u@example.com".to_string(), None, None);
        if password {
            user.set_credentials(UserCredentials::new(
                user.id(),
                "hash".to_string(),
                "argon2id".to_string(),
                true,
            ));
        }
        if verified {
            user.verify_email();
        }
        self.users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());
        user
    }

    fn add_enrollment(&self, enrollment: MfaEnrollment) {
        self.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);
    }

    fn query(&self, user_id: Uuid) -> Query {
        Query {
            tenant_id: self.tenant_id,
            user_id,
        }
    }
}

fn passkey(tenant_id: Uuid, user_id: Uuid, name: &str) -> MfaEnrollment {
    MfaEnrollment::new_webauthn(
        tenant_id,
        user_id,
        json!({"cred": "public key material"}),
        format!("cred-{name}"),
        Some(name.to_string()),
    )
}

#[tokio::test]
async fn a_password_account_with_every_second_factor() {
    let ctx = Ctx::new();
    let user = ctx.add_user(true, true);
    let totp = MfaEnrollment::reconstruct(
        Uuid::now_v7(),
        ctx.tenant_id,
        user.id(),
        MfaMethod::Totp,
        Some("sealed-seed".to_string()),
        Some(1),
        None,
        None,
        None,
        Some(Utc::now()),
        Some(Utc::now()),
        Utc::now(),
    );
    ctx.add_enrollment(totp);
    ctx.add_enrollment(passkey(ctx.tenant_id, user.id(), "Laptop"));
    ctx.users
        .set_preferred_2fa_method(user.id(), Some("totp"))
        .await
        .unwrap();
    let google = SocialIdentity::new(
        user.id(),
        ctx.tenant_id,
        ExternalProviderKind::Google,
        "google-sub".to_string(),
        Some("u@gmail.com".to_string()),
    );
    ctx.socials
        .memory
        .data
        .lock()
        .unwrap()
        .insert(google.id(), google);

    let r = ctx.use_case().execute(ctx.query(user.id())).await.unwrap();

    assert!(r.password.set && r.password.must_change && r.password.changed_at.is_some());
    assert!(
        !r.passkey_sign_in,
        "an account with a password signs in with it"
    );
    assert!(r.magic_link && r.email_verified);
    assert_eq!(r.social_accounts.len(), 1);
    assert_eq!(r.social_accounts[0].provider, "google");
    assert_eq!(r.second_factors.preferred_method.as_deref(), Some("totp"));
    assert!(r.second_factors.email_code);
    assert!(
        r.second_factors
            .authenticator_app
            .as_ref()
            .unwrap()
            .confirmed
    );
    assert_eq!(r.second_factors.passkeys.len(), 1);
    assert_eq!(r.second_factors.passkeys[0].name.as_deref(), Some("Laptop"));

    // Nothing secret leaves the use case.
    let body = serde_json::to_string(&r).unwrap();
    for secret in [
        "sealed-seed",
        "public key material",
        "cred-Laptop",
        "hash",
        "google-sub",
    ] {
        assert!(!body.contains(secret), "{secret} leaked: {body}");
    }
}

#[tokio::test]
async fn a_passwordless_account_with_a_passkey_can_sign_in_with_it_alone() {
    let ctx = Ctx::new();
    let user = ctx.add_user(false, true);
    ctx.add_enrollment(passkey(ctx.tenant_id, user.id(), "Phone"));

    let r = ctx.use_case().execute(ctx.query(user.id())).await.unwrap();

    assert!(!r.password.set && r.password.changed_at.is_none());
    assert!(r.passkey_sign_in);
    assert!(r.second_factors.authenticator_app.is_none());
}

#[tokio::test]
async fn an_unverified_email_offers_neither_email_codes_nor_links() {
    let ctx = Ctx::new();
    let user = ctx.add_user(false, false);
    ctx.add_enrollment(passkey(ctx.tenant_id, user.id(), "Phone"));
    // A pending authenticator app is reported as such.
    ctx.add_enrollment(MfaEnrollment::new_totp(
        ctx.tenant_id,
        user.id(),
        "sealed".to_string(),
    ));

    let r = ctx.use_case().execute(ctx.query(user.id())).await.unwrap();

    assert!(!r.magic_link && !r.second_factors.email_code);
    assert!(
        !r.passkey_sign_in,
        "the login flow requires a verified email"
    );
    assert!(!r.second_factors.authenticator_app.unwrap().confirmed);
}

#[tokio::test]
async fn a_user_of_another_tenant_is_not_found() {
    let ctx = Ctx::new();
    let user = ctx.add_user(true, true);
    let other = Query {
        tenant_id: Uuid::now_v7(),
        user_id: user.id(),
    };
    assert!(matches!(
        ctx.use_case().execute(other).await,
        Err(DomainError::NotFound(_))
    ));
}
