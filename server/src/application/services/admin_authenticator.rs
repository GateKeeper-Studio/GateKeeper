// How a platform admin proves who they are. Every admin sign-in path (the console's
// cookie session and the bearer `/admin/login`) and every step-up on the admin's own
// account goes through here, so they cannot drift apart in how they resist
// enumeration, brute force, or code replay.
//
// - Every failure before the password is proven is the same generic
//   `AuthenticationError`, and costs one Argon2 verification even when there is no
//   such account, so neither the response nor its timing tells which admin emails
//   exist, which are deactivated, or which are locked.
// - Consecutive failures are counted on the account (`MAX_FAILED_SIGN_INS`); at the
//   limit the account is locked for `lockout_duration()`. This is the control that
//   holds against a password spray spread over many IPs, which the per-IP rate
//   limit cannot see.
// - An admin with an enabled second factor needs a TOTP code or a recovery code
//   after the password. Without one, the answer is `SecondFactorRequired`, which
//   is only ever given to a caller that knew the password.
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use uuid::Uuid;

use crate::{
    application::ports::{
        audit_logger::AuditLogger,
        password_hasher::PasswordHasher,
        secret_cipher::{SecretCipher, SecretContext, SecretPurpose},
    },
    domain::{
        entities::{
            admin_mfa::AdminMfa,
            admin_user::{AdminUser, normalize_email},
            audit_action::ADMIN_ACCOUNT_LOCKED,
            audit_event::{ActorType, AuditEvent, Outcome},
        },
        errors::DomainError,
        repositories::{
            admin_mfa_repository::AdminMfaRepository, admin_user_repository::AdminUserRepository,
        },
    },
    utils::{oauth_crypto::sha256_hex, totp},
};

// Consecutive failed sign-ins (wrong password or wrong code) before a lock.
pub const MAX_FAILED_SIGN_INS: i32 = 5;

// How long a locked account refuses every sign-in.
pub fn lockout_duration() -> Duration {
    Duration::minutes(15)
}

// Accepted clock drift between the authenticator app and the server, in 30-second
// steps on either side.
const TOTP_SKEW_STEPS: i64 = 1;

// Recovery codes: how many, and their shape. 10 characters from a 32-symbol
// alphabet without 0/O/1/I is 50 bits each, shown as `XXXXX-XXXXX`.
pub const RECOVERY_CODE_COUNT: usize = 10;
const RECOVERY_CODE_LEN: usize = 10;
const RECOVERY_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

// The wire value of a wrong second factor, so the console can tell it from a wrong
// password.
pub const INVALID_MFA_CODE: &str = "invalid_mfa_code";

pub enum AdminSignIn {
    // Password (and second factor, when enabled) verified.
    Authenticated(AdminUser),
    // The password is right, the account has a second factor, and no code was
    // given. Not a failure: the caller asks for the code and tries again.
    SecondFactorRequired(AdminUser),
}

pub struct AdminAuthenticator {
    admins: Arc<dyn AdminUserRepository>,
    factors: Arc<dyn AdminMfaRepository>,
    hasher: Arc<dyn PasswordHasher>,
    cipher: Arc<dyn SecretCipher>,
    audit_logger: Arc<dyn AuditLogger>,
    // Server-wide Argon2 pepper (`ADMIN_PASSWORD_PEPPER`), the one `create_admin`
    // hashes with.
    pepper: String,
}

impl AdminAuthenticator {
    pub fn new(
        admins: Arc<dyn AdminUserRepository>,
        factors: Arc<dyn AdminMfaRepository>,
        hasher: Arc<dyn PasswordHasher>,
        cipher: Arc<dyn SecretCipher>,
        audit_logger: Arc<dyn AuditLogger>,
        pepper: String,
    ) -> Self {
        Self {
            admins,
            factors,
            hasher,
            cipher,
            audit_logger,
            pepper,
        }
    }

    pub async fn sign_in(
        &self,
        email: &str,
        password: &str,
        second_factor: Option<&str>,
    ) -> Result<AdminSignIn, DomainError> {
        let now = Utc::now();
        let invalid = || DomainError::AuthenticationError("invalid credentials".to_string());

        let admin = match self.admins.find_by_email(&normalize_email(email)).await? {
            Some(admin) if admin.can_sign_in() && !admin.is_locked(now) => admin,
            _ => {
                self.hasher.verify_dummy(password).await;
                return Err(invalid());
            }
        };

        if !self
            .hasher
            .verify(admin.password_hash(), password, &self.pepper)
            .await
        {
            self.register_failure(&admin, now).await?;
            return Err(invalid());
        }

        if let Some(factor) = self.enabled_factor(admin.id()).await? {
            let Some(code) = second_factor.map(str::trim).filter(|c| !c.is_empty()) else {
                return Ok(AdminSignIn::SecondFactorRequired(admin));
            };
            if !self.verify_code(&factor, code, now).await? {
                self.register_failure(&admin, now).await?;
                return Err(DomainError::MfaError(INVALID_MFA_CODE.to_string()));
            }
        }

        self.admins
            .record_successful_sign_in(admin.id(), now)
            .await?;
        Ok(AdminSignIn::Authenticated(admin))
    }

    // Step-up on the admin's own account: the current password must be presented.
    // `None` asks for it (`step_up_required`); a wrong one is a 400 the console shows
    // inline. A wrong password counts toward the lock, so a hijacked session cannot
    // be used to guess the password either.
    pub async fn require_password(
        &self,
        admin: &AdminUser,
        password: Option<&str>,
    ) -> Result<(), DomainError> {
        let Some(password) = password.filter(|p| !p.is_empty()) else {
            return Err(DomainError::StepUpRequired(
                "enter your current password to continue".to_string(),
            ));
        };
        let now = Utc::now();
        if admin.is_locked(now) {
            self.hasher.verify_dummy(password).await;
            return Err(DomainError::Validation(
                "current password is incorrect".to_string(),
            ));
        }
        if !self
            .hasher
            .verify(admin.password_hash(), password, &self.pepper)
            .await
        {
            self.register_failure(admin, now).await?;
            return Err(DomainError::Validation(
                "current password is incorrect".to_string(),
            ));
        }
        Ok(())
    }

    // Step-up with the second factor: a current TOTP code or an unused recovery
    // code (which is then used up). A wrong code counts toward the lock.
    pub async fn require_second_factor(
        &self,
        admin: &AdminUser,
        code: Option<&str>,
    ) -> Result<(), DomainError> {
        let now = Utc::now();
        let Some(factor) = self.enabled_factor(admin.id()).await? else {
            return Err(DomainError::Validation(
                "two-step verification is not enabled".to_string(),
            ));
        };
        let code = code.map(str::trim).unwrap_or_default();
        if code.is_empty() || !self.verify_code(&factor, code, now).await? {
            self.register_failure(admin, now).await?;
            return Err(DomainError::Validation(
                "the verification code is not valid".to_string(),
            ));
        }
        Ok(())
    }

    // Whether the admin's organization requires two-step verification and this admin
    // has not turned it on. Such an admin is signed in but restricted to enrolling
    // (the token carries `mfa_setup`, which `require_admin` enforces). Costs a query
    // only in organizations that require it.
    pub async fn mfa_setup_required(&self, admin: &AdminUser) -> Result<bool, DomainError> {
        if !admin.organization_requires_mfa() {
            return Ok(false);
        }
        Ok(self.enabled_factor(admin.id()).await?.is_none())
    }

    // Whether the admin has two-step verification on.
    pub async fn has_second_factor(&self, admin_id: Uuid) -> Result<bool, DomainError> {
        Ok(self.enabled_factor(admin_id).await?.is_some())
    }

    pub async fn hash_password(&self, password: &str) -> Result<String, DomainError> {
        self.hasher.hash(password, &self.pepper).await
    }

    // Seals a new TOTP seed for `admin_id`.
    pub fn seal_totp_seed(&self, admin_id: Uuid, seed_base32: &str) -> Result<String, DomainError> {
        self.cipher.encrypt(seed_base32, seed_context(admin_id))
    }

    // The TOTP step `code` matches for this factor's seed, without recording it.
    // `None` for a wrong code or an unreadable seed.
    pub fn match_totp(&self, factor: &AdminMfa, code: &str, now: DateTime<Utc>) -> Option<i64> {
        let sealed = factor.totp_secret_enc();
        let seed = match self.cipher.decrypt(sealed, seed_context(factor.admin_id())) {
            Ok(seed) => seed,
            Err(_) => {
                tracing::error!(
                    admin_id = %factor.admin_id(),
                    "admin TOTP seed cannot be decrypted (run `repair_encrypted_secrets`)"
                );
                return None;
            }
        };
        let secret = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &seed)?;
        totp::verify(&secret, code, now.timestamp(), TOTP_SKEW_STEPS)
    }

    async fn enabled_factor(&self, admin_id: Uuid) -> Result<Option<AdminMfa>, DomainError> {
        Ok(self
            .factors
            .find(admin_id)
            .await?
            .filter(AdminMfa::is_enabled))
    }

    // A 6-digit code is tried as TOTP; anything else as a recovery code.
    async fn verify_code(
        &self,
        factor: &AdminMfa,
        code: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        if code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit()) {
            let Some(step) = self.match_totp(factor, code, now) else {
                return Ok(false);
            };
            if !factor.accepts_step(step) {
                return Ok(false);
            }
            // Conditional write: of two requests with the same code, one wins.
            return self
                .factors
                .record_totp_step(factor.admin_id(), step, now)
                .await;
        }

        let normalized = normalize_recovery_code(code);
        if normalized.len() != RECOVERY_CODE_LEN {
            return Ok(false);
        }
        self.factors
            .consume_recovery_code(factor.admin_id(), &sha256_hex(&normalized), now)
            .await
    }

    async fn register_failure(
        &self,
        admin: &AdminUser,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let locked = self
            .admins
            .register_failed_sign_in(admin.id(), MAX_FAILED_SIGN_INS, now + lockout_duration())
            .await?;
        if locked {
            tracing::warn!(admin_id = %admin.id(), "admin account locked after failed sign-ins");
            let event = AuditEvent::new(
                None,
                ActorType::Admin,
                Some(admin.id()),
                ADMIN_ACCOUNT_LOCKED,
                Outcome::Failure,
            )
            .with_resource("admin", admin.id())
            .with_metadata(serde_json::json!({
                "failed_attempts": MAX_FAILED_SIGN_INS,
                "locked_minutes": lockout_duration().num_minutes(),
            }));
            let _ = self.audit_logger.record(event).await;
        }
        Ok(())
    }
}

fn seed_context(admin_id: Uuid) -> SecretContext {
    SecretContext::new(SecretPurpose::AdminTotpSeed, admin_id)
}

// Uppercase, without the separator and any spaces, so `abcde-fghij` matches.
fn normalize_recovery_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

// A fresh set of recovery codes: the codes to show once, and the hashes to store.
pub fn generate_recovery_codes() -> (Vec<String>, Vec<String>) {
    let mut rng = rand::rng();
    let mut codes = Vec::with_capacity(RECOVERY_CODE_COUNT);
    let mut hashes = Vec::with_capacity(RECOVERY_CODE_COUNT);
    for _ in 0..RECOVERY_CODE_COUNT {
        let raw: String = (0..RECOVERY_CODE_LEN)
            .map(|_| RECOVERY_ALPHABET[rng.random_range(0..RECOVERY_ALPHABET.len())] as char)
            .collect();
        hashes.push(sha256_hex(&raw));
        codes.push(format!("{}-{}", &raw[..5], &raw[5..]));
    }
    (codes, hashes)
}

#[cfg(test)]
mod tests;
