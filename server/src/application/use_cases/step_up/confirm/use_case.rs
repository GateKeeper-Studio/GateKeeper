use std::sync::Arc;

use chrono::Utc;
use subtle::ConstantTimeEq;

use crate::{
    application::{
        ports::{
            password_hasher::PasswordHasher, unit_of_work::UnitOfWorkFactory,
            webauthn::WebauthnService,
        },
        use_cases::step_up::confirm::command::{Command, Requirement},
    },
    domain::{
        entities::{mfa_challenge::MfaChallengePurpose, mfa_method::MfaMethod},
        errors::DomainError,
    },
    utils::oauth_crypto::sha256_hex,
};

// The `error_description` of a missing proof names the proof, so a client knows
// what to collect: `current_password`, or a `step_up` from `POST /me/step-up`.
pub const PASSWORD_REQUIRED: &str = "this action requires `current_password`";
pub const STEP_UP_REQUIRED: &str =
    "this action requires a step-up: call POST /me/step-up and send its proof as `step_up`";
pub const WRONG_PASSWORD: &str = "current password is incorrect";
pub const WRONG_PROOF: &str = "the step-up proof is invalid or expired";

// Fresh proof that the caller is the account's owner, checked before a sensitive
// self-service action, so a stolen access token alone can do none of them:
//   - with a password: `current_password` (never one awaiting a forced change);
//   - without one: the passkey assertion or emailed code of a `step_up/start`
//     ceremony. Single use; a wrong proof counts against the ceremony
//     (MfaChallenge::MAX_ATTEMPTS) and is persisted before the refusal.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    password_hasher: Arc<dyn PasswordHasher>,
    webauthn: Arc<dyn WebauthnService>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        password_hasher: Arc<dyn PasswordHasher>,
        webauthn: Arc<dyn WebauthnService>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            password_hasher,
            webauthn,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        let user = uow
            .user_repository()
            .find_by_id_with_credentials(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id)
            .ok_or_else(|| DomainError::AccessDenied("account not found".to_string()))?;

        if let Some(credentials) = user.credentials() {
            if command.requirement == Requirement::WithoutPassword {
                return Ok(());
            }
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
            // A password awaiting a forced change (an admin reset's, perhaps already
            // used) is not the user's: it confirms nothing.
            return if matches && credentials.proves_account_owner() {
                Ok(())
            } else {
                Err(DomainError::StepUpRequired(WRONG_PASSWORD.to_string()))
            };
        }

        // No password. The proof is the passkey when the account has one, otherwise
        // the emailed code: the same choice `step_up/start` made.
        let mut enrollments: Vec<_> = uow
            .mfa_enrollment_repository()
            .find_confirmed_by_user_id(command.user_id)
            .await?
            .into_iter()
            .filter(|e| e.tenant_id() == command.tenant_id && *e.method() == MfaMethod::Webauthn)
            .collect();
        let passkeys: Vec<_> = enrollments
            .iter()
            .filter_map(|e| e.passkey_json().cloned())
            .collect();
        let expected = if passkeys.is_empty() {
            MfaMethod::Email
        } else {
            MfaMethod::Webauthn
        };

        let proof = command
            .proof
            .ok_or_else(|| DomainError::StepUpRequired(STEP_UP_REQUIRED.to_string()))?;
        let wrong = || DomainError::StepUpRequired(WRONG_PROOF.to_string());
        let mut challenge = uow
            .mfa_challenge_repository()
            .find_by_id(command.tenant_id, proof.challenge_id)
            .await?
            .filter(|c| {
                c.user_id() == command.user_id
                    && *c.purpose() == MfaChallengePurpose::StepUp
                    && *c.method() == expected
                    && c.is_redeemable(now)
            })
            .ok_or_else(wrong)?;

        let verified = match expected {
            MfaMethod::Webauthn => {
                let state = challenge.state_json().cloned().ok_or_else(wrong)?;
                proof
                    .assertion
                    .as_ref()
                    .ok_or_else(wrong)
                    .and_then(|assertion| {
                        self.webauthn
                            .finish_authentication(assertion, &state, &passkeys)
                    })
                    .and_then(|authenticated| {
                        enrollments
                            .iter()
                            .position(|e| {
                                e.credential_id() == Some(authenticated.credential_id.as_str())
                            })
                            .map(|index| Some((index, authenticated.updated_passkey)))
                            .ok_or_else(wrong)
                    })
            }
            _ => {
                let expected_hash = challenge.code_hash().unwrap_or_default().to_string();
                let submitted = sha256_hex(proof.code.as_deref().unwrap_or_default().trim());
                if proof.code.is_some()
                    && expected_hash
                        .as_bytes()
                        .ct_eq(submitted.as_bytes())
                        .unwrap_u8()
                        == 1
                {
                    Ok(None)
                } else {
                    Err(wrong())
                }
            }
        };

        let matched = match verified {
            Ok(matched) => matched,
            Err(_) => {
                // A wrong proof is not free: counted and persisted before answering.
                challenge.register_failed_attempt();
                uow.mfa_challenge_repository().update(&challenge).await?;
                uow.commit().await?;
                return Err(wrong());
            }
        };

        // Clone detection depends on the signature counter never going backwards.
        if let Some((index, Some(updated))) = matched {
            let enrollment = &mut enrollments[index];
            enrollment.record_webauthn_use(updated, now);
            uow.mfa_enrollment_repository().update(enrollment).await?;
        }
        challenge.consume(now);
        uow.mfa_challenge_repository().update(&challenge).await?;
        uow.commit().await?;
        Ok(())
    }
}
