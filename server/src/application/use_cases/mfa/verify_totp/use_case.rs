use std::sync::Arc;

use chrono::Utc;

use crate::application::ports::secret_cipher::{SecretContext, SecretPurpose};
use crate::application::shared::risk_feedback::{label_if_mfa_exhausted, label_mfa_passed};
use crate::{
    application::{
        ports::{secret_cipher::SecretCipher, unit_of_work::UnitOfWorkFactory},
        use_cases::mfa::verify_totp::{command::Command, response::Response},
    },
    domain::{
        entities::{
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_method::MfaMethod,
        },
        errors::DomainError,
    },
    utils::{oauth_crypto::sha256_hex, totp},
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    secret_cipher: Arc<dyn SecretCipher>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        secret_cipher: Arc<dyn SecretCipher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            secret_cipher,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();
        let invalid = || DomainError::AuthenticationError("invalid MFA challenge".to_string());

        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active() && c.allows_mfa_method(&MfaMethod::Totp))
            .ok_or_else(invalid)?;

        let mut session = uow
            .auth_session_repository()
            .find_by_token_hash(client.tenant_id(), &sha256_hex(&command.session_code))
            .await?
            .ok_or_else(invalid)?;

        if session.is_expired(now)
            || !session.is_authenticated()
            || !session.is_mfa_pending()
            // A passkey (passwordless) session can only be opened by WebAuthn: a TOTP code
            // alone must never act as a primary factor.
            || !session.accepts_mfa_method(&MfaMethod::Totp)
            || session.pending_client_id() != client.id()
        {
            return Err(invalid());
        }

        let user_id = session.user_id().ok_or_else(invalid)?;
        let mut enrollment = match command.mfa_id {
            Some(id) => {
                uow.mfa_enrollment_repository()
                    .find_by_id(client.tenant_id(), id)
                    .await?
            }
            None => {
                uow.mfa_enrollment_repository()
                    .find_totp_by_user_id(user_id)
                    .await?
            }
        }
        .filter(|e| e.user_id() == user_id && *e.method() == MfaMethod::Totp && e.is_confirmed())
        .ok_or_else(invalid)?;

        // Cap wrong tries. TOTP has no server-issued code, so there was no challenge
        // row bounding attempts the way email OTP does (MAX_ATTEMPTS = 5) — a caller
        // holding a valid session code could brute-force the 6-digit space. The
        // counter is created lazily on the first attempt for this session.
        let mut challenge = match uow
            .mfa_challenge_repository()
            .find_active_by_session(session.id(), &MfaMethod::Totp)
            .await?
            .filter(|c| *c.purpose() == MfaChallengePurpose::Login)
        {
            Some(existing) => {
                // Exhausted or expired: refuse without even checking the code, so a
                // burned counter cannot be outlived by guessing.
                if !existing.is_redeemable(now) {
                    return Err(invalid());
                }
                existing
            }
            None => {
                let created = MfaChallenge::new_totp_attempt_counter(
                    client.tenant_id(),
                    user_id,
                    Some(session.id()),
                    MfaChallengePurpose::Login,
                    session.expires_at(),
                );
                uow.mfa_challenge_repository().add(&created).await?;
                created
            }
        };

        // From here every failure path must persist the incremented counter, so the
        // attempt is not free.
        let fail = async |uow: Box<dyn crate::application::ports::unit_of_work::UnitOfWork>,
                          challenge: &mut MfaChallenge|
               -> Result<(), DomainError> {
            challenge.register_failed_attempt();
            uow.mfa_challenge_repository().update(challenge).await?;
            label_if_mfa_exhausted(&*uow, &session, challenge).await?;
            uow.commit().await?;
            Ok(())
        };

        let encrypted = match enrollment.secret_enc() {
            Some(encrypted) => encrypted.to_string(),
            None => {
                fail(uow, &mut challenge).await?;
                return Err(invalid());
            }
        };
        let secret_base32 = self.secret_cipher.decrypt(
            &encrypted,
            SecretContext::new(SecretPurpose::TotpSeed, enrollment.tenant_id()),
        )?;
        let secret =
            match base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret_base32) {
                Some(secret) => secret,
                None => {
                    fail(uow, &mut challenge).await?;
                    return Err(invalid());
                }
            };

        let matched_step = match totp::verify(&secret, command.code.trim(), now.timestamp(), 1) {
            Some(step) => step,
            None => {
                fail(uow, &mut challenge).await?;
                return Err(invalid());
            }
        };

        // Replay of an already-used step counts as a failed attempt too.
        if enrollment
            .totp_last_step()
            .map(|last| matched_step <= last)
            .unwrap_or(false)
        {
            fail(uow, &mut challenge).await?;
            return Err(invalid());
        }

        challenge.consume(now);
        enrollment.record_totp_use(matched_step, now);
        session.verify_mfa(&MfaMethod::Totp, now);
        uow.mfa_challenge_repository().update(&challenge).await?;
        uow.mfa_enrollment_repository().update(&enrollment).await?;
        uow.auth_session_repository().update(&session).await?;
        label_mfa_passed(&*uow, &session).await?;
        uow.commit().await?;

        Ok(Response {
            session_code: command.session_code,
        })
    }
}
