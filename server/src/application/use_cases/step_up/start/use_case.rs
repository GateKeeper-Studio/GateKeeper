use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::shared::token_policy::{
    MFA_CODE_TTL_MINUTES, WEBAUTHN_CHALLENGE_TTL_MINUTES,
};
use crate::{
    application::{
        ports::{unit_of_work::UnitOfWorkFactory, webauthn::WebauthnService},
        services::templated_email_sender::TemplatedEmailSender,
        use_cases::step_up::start::{command::Command, response::Response},
    },
    domain::{
        entities::{
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_method::MfaMethod,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_numeric_code, sha256_hex},
};

// Emailed codes per account and window. Each code brings a fresh set of wrong tries
// (MfaChallenge::MAX_ATTEMPTS), so this bounds the guesses as much as the email.
pub const MAX_CODES_PER_WINDOW: i64 = 3;
pub const CODE_WINDOW_MINUTES: i64 = 15;

// Opens the step-up an account without a password must pass before a sensitive
// action. A new ceremony replaces any earlier one of the same kind (one live proof).
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    webauthn: Arc<dyn WebauthnService>,
    templated_email: Arc<TemplatedEmailSender>,
    // Outside development an email with no provider is dropped: refuse instead of
    // waiting for a code that never comes.
    require_email_provider: bool,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        webauthn: Arc<dyn WebauthnService>,
        templated_email: Arc<TemplatedEmailSender>,
        require_email_provider: bool,
    ) -> Self {
        Self {
            unit_of_work_factory,
            webauthn,
            templated_email,
            require_email_provider,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        let user = uow
            .user_repository()
            .find_by_id_with_credentials(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id && u.is_active())
            .ok_or_else(|| DomainError::AccessDenied("account not found".to_string()))?;

        // The password is the proof; there is nothing to start.
        if user.credentials().is_some() {
            return Ok(Response::Password);
        }

        let passkeys: Vec<_> = uow
            .mfa_enrollment_repository()
            .find_confirmed_by_user_id(command.user_id)
            .await?
            .into_iter()
            .filter(|e| e.tenant_id() == command.tenant_id && *e.method() == MfaMethod::Webauthn)
            .filter_map(|e| e.passkey_json().cloned())
            .collect();

        // An account with a passkey proves it with the passkey: the strongest proof it
        // has, and the one a stolen token cannot produce.
        if !passkeys.is_empty() {
            let ceremony = self.webauthn.start_authentication(&passkeys)?;
            uow.mfa_challenge_repository()
                .invalidate_active_by_user(
                    command.user_id,
                    &MfaChallengePurpose::StepUp,
                    &MfaMethod::Webauthn,
                )
                .await?;
            let challenge = MfaChallenge::new_webauthn(
                command.tenant_id,
                command.user_id,
                None,
                MfaChallengePurpose::StepUp,
                ceremony.state,
                now + Duration::minutes(WEBAUTHN_CHALLENGE_TTL_MINUTES),
            );
            let challenge_id = challenge.id();
            uow.mfa_challenge_repository().add(&challenge).await?;
            uow.commit().await?;
            return Ok(Response::Passkey {
                challenge_id,
                options: ceremony.options,
            });
        }

        // No password and no passkey (a social account): a code to the address on
        // file. Only a verified one: an unproven address may not be the user's.
        if !user.is_email_verified() {
            return Err(DomainError::Validation(
                "confirm your email address before making this change".to_string(),
            ));
        }
        let issued = uow
            .mfa_challenge_repository()
            .count_issued_since(
                command.user_id,
                &MfaChallengePurpose::StepUp,
                &MfaMethod::Email,
                now - Duration::minutes(CODE_WINDOW_MINUTES),
            )
            .await?;
        if issued >= MAX_CODES_PER_WINDOW {
            return Err(DomainError::Validation(
                "too many codes requested; try again in a few minutes".to_string(),
            ));
        }

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| DomainError::AccessDenied("account not found".to_string()))?;
        let email_provider = uow
            .email_provider_repository()
            .find_default_by_tenant_id(command.tenant_id)
            .await?
            .filter(|p| p.is_active());
        if email_provider.is_none() && self.require_email_provider {
            return Err(DomainError::Validation(
                "this tenant has no active email provider, so the code could not be sent"
                    .to_string(),
            ));
        }

        let code = generate_numeric_code();
        let expires_at = now + Duration::minutes(MFA_CODE_TTL_MINUTES);
        uow.mfa_challenge_repository()
            .invalidate_active_by_user(
                command.user_id,
                &MfaChallengePurpose::StepUp,
                &MfaMethod::Email,
            )
            .await?;
        let challenge = MfaChallenge::new_email_otp(
            command.tenant_id,
            command.user_id,
            None,
            MfaChallengePurpose::StepUp,
            sha256_hex(&code),
            expires_at,
        );
        let challenge_id = challenge.id();
        uow.mfa_challenge_repository().add(&challenge).await?;
        uow.commit().await?;

        // After the commit, so the code that arrives is one that works.
        self.templated_email
            .send_step_up_code(
                command.tenant_id,
                email_provider.as_ref(),
                user.email(),
                &code,
                tenant.name(),
                &format!("{MFA_CODE_TTL_MINUTES} minutes"),
            )
            .await?;

        Ok(Response::Email {
            challenge_id,
            expires_at,
        })
    }
}
