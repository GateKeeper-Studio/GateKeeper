// A platform admin with everything the admin use cases need around it: in-memory
// repositories, the real Argon2 hasher, the stub cipher, and an `AdminAuthenticator`
// wired to all of them. One place, so each test states only what it is about.
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::services::admin_authenticator::{AdminAuthenticator, generate_recovery_codes},
    domain::{
        entities::{admin_mfa::AdminMfa, admin_user::AdminUser},
        repositories::admin_mfa_repository::AdminMfaRepository,
    },
    testing::{
        fake_audit_logger::FakeAuditLogger,
        in_memory_repositories::{
            in_memory_admin_mfa_repository::InMemoryAdminMfaRepository,
            in_memory_admin_refresh_token_repository::InMemoryAdminRefreshTokenRepository,
            in_memory_admin_user_repository::InMemoryAdminUserRepository,
            in_memory_admin_web_session_repository::InMemoryAdminWebSessionRepository,
        },
        password_hasher::test_password_hasher,
        stub_secret_cipher::StubSecretCipher,
    },
    utils::{password_hasher::hash_password, totp},
};

pub const PEPPER: &str = "test-pepper";
pub const PASSWORD: &str = "current-password";
pub const EMAIL: &str = "admin@example.com";
// The TOTP seed `with_totp` enrolls, base32.
pub const TOTP_SEED: &str = "JBSWY3DPEHPK3PXP";

pub struct AdminFixture {
    pub admins: Arc<InMemoryAdminUserRepository>,
    pub factors: Arc<InMemoryAdminMfaRepository>,
    pub refresh_tokens: Arc<InMemoryAdminRefreshTokenRepository>,
    pub web_sessions: Arc<InMemoryAdminWebSessionRepository>,
    pub audit: Arc<FakeAuditLogger>,
    pub authenticator: Arc<AdminAuthenticator>,
    pub admin: AdminUser,
}

impl AdminFixture {
    // One active admin (`EMAIL` / `PASSWORD`) alone in its organization.
    pub fn new() -> Self {
        let admins = Arc::new(InMemoryAdminUserRepository::new());
        let factors = Arc::new(InMemoryAdminMfaRepository::new());
        let audit = Arc::new(FakeAuditLogger::new());
        let authenticator = Arc::new(AdminAuthenticator::new(
            admins.clone(),
            factors.clone(),
            test_password_hasher(),
            Arc::new(StubSecretCipher),
            audit.clone(),
            PEPPER.to_string(),
        ));
        let admin = AdminUser::new(
            Uuid::now_v7(),
            EMAIL.to_string(),
            "Ada Admin".to_string(),
            hash_password(PASSWORD, PEPPER).unwrap(),
            "argon2id".to_string(),
        );
        admins.admins.lock().unwrap().push(admin.clone());
        Self {
            admins,
            factors,
            refresh_tokens: Arc::new(InMemoryAdminRefreshTokenRepository::new()),
            web_sessions: Arc::new(InMemoryAdminWebSessionRepository::new()),
            audit,
            authenticator,
            admin,
        }
    }

    // Adds another active admin to the same organization.
    pub fn add_colleague(&self, email: &str) -> AdminUser {
        let colleague = AdminUser::new(
            self.admin.organization_id(),
            email.to_string(),
            "Colleague".to_string(),
            hash_password(PASSWORD, PEPPER).unwrap(),
            "argon2id".to_string(),
        );
        self.admins.admins.lock().unwrap().push(colleague.clone());
        colleague
    }

    // Enrolls and confirms an authenticator app (seed `TOTP_SEED`). Returns the
    // recovery codes.
    pub async fn with_totp(&self) -> Vec<String> {
        let now = Utc::now();
        let pending = AdminMfa::pending(self.admin.id(), format!("enc:{TOTP_SEED}"), now);
        assert!(self.factors.save_pending(&pending).await.unwrap());
        let (codes, hashes) = generate_recovery_codes();
        // Confirmed at an old step, so the current code is accepted.
        assert!(
            self.factors
                .confirm(self.admin.id(), 0, &hashes, now)
                .await
                .unwrap()
        );
        codes
    }

    // The current TOTP code for `TOTP_SEED`.
    pub fn totp_code(&self) -> String {
        current_code(TOTP_SEED)
    }

    // The admin as stored now.
    pub fn stored(&self) -> AdminUser {
        self.admins
            .admins
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.id() == self.admin.id())
            .cloned()
            .unwrap()
    }
}

impl Default for AdminFixture {
    fn default() -> Self {
        Self::new()
    }
}

// An authenticator over an existing admin store, for tests that build their own
// repositories. No admin in it has a second factor.
pub fn authenticator_for(admins: Arc<InMemoryAdminUserRepository>) -> Arc<AdminAuthenticator> {
    Arc::new(AdminAuthenticator::new(
        admins,
        Arc::new(InMemoryAdminMfaRepository::new()),
        test_password_hasher(),
        Arc::new(StubSecretCipher),
        Arc::new(FakeAuditLogger::new()),
        PEPPER.to_string(),
    ))
}

// The current TOTP code for a base32 seed.
pub fn current_code(seed_base32: &str) -> String {
    let secret = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, seed_base32).unwrap();
    totp::generate(&secret, totp::step_for(Utc::now().timestamp()))
}
