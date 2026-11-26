use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::{
    application::{
        ports::{
            password_hasher::PasswordHasher,
            secret_cipher::{SecretCipher, SecretContext, SecretPurpose},
            unit_of_work::{UnitOfWork, UnitOfWorkFactory},
        },
        use_cases::trusted_device::{
            register::{command::Command, response::Response},
            shared::registration_message,
        },
    },
    domain::{
        entities::{
            audit_action::USER_TRUSTED_DEVICE_REGISTERED,
            audit_event::{ActorType, AuditEvent, Outcome},
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_enrollment::MfaEnrollment,
            mfa_method::MfaMethod,
            trusted_device::{MAX_PER_USER, TrustedDevice},
        },
        errors::DomainError,
    },
    utils::{device_key, totp},
};

// POST /me/trusted-devices — an app the user is signed in to registers its device key.
//
// A trusted device is a new way to approve sign-ins, so registering one needs more
// than the access token (which a thief may hold): the account's current password, and
// a signature showing the app holds the private key. With a current authenticator-app
// code as well — or for a user who has no second factor yet — the device's approvals
// count as a second factor; otherwise the browser it approves is still asked for one.
//
// The route sits behind the strict rate limit: the password is checked here.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    password_hasher: Arc<dyn PasswordHasher>,
    secret_cipher: Arc<dyn SecretCipher>,
}

const NO_PASSWORD: &str = "an account without a password cannot register a trusted device";
const PASSWORD_REQUIRED: &str = "registering a trusted device requires `currentPassword`";
const WRONG_PASSWORD: &str = "current password is incorrect";
const TOO_MANY_WRONG_CODES: &str = "too many wrong authenticator codes: try again later";
// After `MfaChallenge::MAX_ATTEMPTS` wrong codes, how long no code is accepted here.
const TOTP_LOCKOUT_MINUTES: i64 = 15;

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        password_hasher: Arc<dyn PasswordHasher>,
        secret_cipher: Arc<dyn SecretCipher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            password_hasher,
            secret_cipher,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        // 1. The account, and its password.
        let user = uow
            .user_repository()
            .find_by_id_with_credentials(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id && u.is_active())
            .ok_or_else(|| DomainError::AccessDenied("account not found".to_string()))?;
        let credentials = user
            .credentials()
            .ok_or_else(|| DomainError::AccessDenied(NO_PASSWORD.to_string()))?;
        let password = command
            .current_password
            .as_deref()
            .filter(|p| !p.is_empty())
            .ok_or_else(|| DomainError::StepUpRequired(PASSWORD_REQUIRED.to_string()))?;
        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| DomainError::AccessDenied("account not found".to_string()))?;
        let matches = self
            .password_hasher
            .verify(
                credentials.password_hash(),
                password,
                tenant.password_hash_secret(),
            )
            .await;
        // A password awaiting a forced change is not the user's.
        if !matches || !credentials.proves_account_owner() {
            return Err(DomainError::StepUpRequired(WRONG_PASSWORD.to_string()));
        }

        // 2. The device: a P-256 key, and proof the app holds its private half.
        let name = TrustedDevice::normalize_name(&command.name).ok_or_else(|| {
            DomainError::Validation("name must be 1 to 64 characters".to_string())
        })?;
        let public_key =
            device_key::normalize_public_key(&command.public_key).ok_or_else(|| {
                DomainError::Validation("publicKey must be an ES256 (P-256) public key".to_string())
            })?;
        let message = registration_message(command.user_id, &command.public_key);
        if !device_key::verify(&public_key, message.as_bytes(), &command.signature) {
            return Err(DomainError::Validation(
                "signature does not prove possession of the key".to_string(),
            ));
        }
        let devices = uow.trusted_device_repository();
        if devices.has_key(command.user_id, &public_key).await? {
            return Err(DomainError::Conflict(
                "this device is already trusted".to_string(),
            ));
        }
        if devices
            .count_for_user(command.tenant_id, command.user_id)
            .await?
            >= MAX_PER_USER
        {
            return Err(DomainError::Conflict(format!(
                "at most {MAX_PER_USER} trusted devices: remove one first"
            )));
        }

        // 3. Does it count as a second factor?
        let counts_as_second_factor = match command.mfa_code.as_deref().map(str::trim) {
            Some(code) if !code.is_empty() => {
                if let Err(refused) = self.verify_totp(uow.as_ref(), &command, code, now).await? {
                    // Keep the wrong try: it is the only write of this unit of work.
                    uow.commit().await?;
                    return Err(refused);
                }
                true
            }
            // A user with no second factor at all gains one: the device.
            _ => uow
                .mfa_enrollment_repository()
                .find_confirmed_by_user_id(command.user_id)
                .await?
                .is_empty(),
        };

        let device = TrustedDevice::new(
            command.tenant_id,
            command.user_id,
            name,
            public_key,
            counts_as_second_factor,
            now,
        );
        devices.add(&device).await?;
        uow.audit_event_repository()
            .record(
                &AuditEvent::new(
                    Some(command.tenant_id),
                    ActorType::User,
                    Some(command.user_id),
                    USER_TRUSTED_DEVICE_REGISTERED,
                    Outcome::Success,
                )
                .with_request_context(command.ip_address.clone(), command.user_agent.clone())
                .with_metadata(serde_json::json!({
                    "device_id": device.id(),
                    "name": device.name(),
                    "counts_as_second_factor": counts_as_second_factor,
                })),
            )
            .await?;
        uow.commit().await?;

        Ok(Response::from(&device))
    }

    // A current code from the user's authenticator app, with the same replay guard as
    // sign-in: a step already used is refused, and this one is recorded.
    // Checks the authenticator code. The outer error is a fault; the inner one is a
    // refusal the caller answers with after committing the wrong try recorded here.
    //
    // SECURITY: the code has six digits, so it gets the same five wrong tries as at
    // sign-in. Without the counter, whoever held the password and a token could guess
    // it and register a device that counts as a second factor. The enrollment row is
    // read under a lock (`find_totp_by_user_id`), so parallel guesses are each counted.
    async fn verify_totp(
        &self,
        uow: &dyn UnitOfWork,
        command: &Command,
        code: &str,
        now: DateTime<Utc>,
    ) -> Result<Result<(), DomainError>, DomainError> {
        let invalid = || DomainError::MfaError("invalid authenticator code".to_string());
        let mut enrollment = uow
            .mfa_enrollment_repository()
            .find_totp_by_user_id(command.user_id)
            .await?
            .filter(|e| e.tenant_id() == command.tenant_id && e.is_confirmed())
            .ok_or_else(|| {
                DomainError::Validation("no authenticator app is enrolled".to_string())
            })?;

        let challenges = uow.mfa_challenge_repository();
        let mut counter = match challenges
            .find_active_by_user(
                command.user_id,
                &MfaChallengePurpose::Enroll,
                &MfaMethod::Totp,
            )
            .await?
            .filter(|c| !c.is_expired(now))
        {
            Some(counter) => counter,
            None => {
                let counter = MfaChallenge::new_totp_attempt_counter(
                    command.tenant_id,
                    command.user_id,
                    None,
                    MfaChallengePurpose::Enroll,
                    now + Duration::minutes(TOTP_LOCKOUT_MINUTES),
                );
                challenges.add(&counter).await?;
                counter
            }
        };
        if counter.attempts_exhausted() {
            return Ok(Err(DomainError::MfaError(TOO_MANY_WRONG_CODES.to_string())));
        }

        let step = match self.matching_step(&enrollment, code, now.timestamp())? {
            Some(step) => step,
            None => {
                counter.register_failed_attempt();
                challenges.update(&counter).await?;
                return Ok(Err(invalid()));
            }
        };

        counter.consume(now);
        challenges.update(&counter).await?;
        enrollment.record_totp_use(step, now);
        uow.mfa_enrollment_repository().update(&enrollment).await?;
        Ok(Ok(()))
    }

    // The time step `code` is valid for, or None for a wrong or already used code.
    fn matching_step(
        &self,
        enrollment: &MfaEnrollment,
        code: &str,
        unix_seconds: i64,
    ) -> Result<Option<i64>, DomainError> {
        let Some(encrypted) = enrollment.secret_enc() else {
            return Ok(None);
        };
        let seed = self.secret_cipher.decrypt(
            encrypted,
            SecretContext::new(SecretPurpose::TotpSeed, enrollment.tenant_id()),
        )?;
        let Some(secret) = base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &seed)
        else {
            return Ok(None);
        };
        Ok(totp::verify(&secret, code, unix_seconds, 1)
            .filter(|step| enrollment.totp_last_step().is_none_or(|last| *step > last)))
    }
}
