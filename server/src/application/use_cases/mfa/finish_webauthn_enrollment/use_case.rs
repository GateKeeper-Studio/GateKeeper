use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::{unit_of_work::UnitOfWorkFactory, webauthn::WebauthnService},
        use_cases::mfa::finish_webauthn_enrollment::{command::Command, response::Response},
    },
    domain::{
        entities::{
            mfa_challenge::MfaChallengePurpose, mfa_enrollment::MfaEnrollment,
            mfa_method::MfaMethod,
        },
        errors::DomainError,
    },
};

// mfa_enrollments.name is VARCHAR(100).
const MAX_NAME_LENGTH: usize = 100;

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    webauthn: Arc<dyn WebauthnService>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        webauthn: Arc<dyn WebauthnService>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            webauthn,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();
        let invalid =
            || DomainError::InvalidRequest("invalid or expired enrollment challenge".to_string());

        let name = normalize_name(command.name)?;

        let mut challenge = uow
            .mfa_challenge_repository()
            .find_by_id(command.tenant_id, command.mfa_id)
            .await?
            .filter(|c| {
                c.user_id() == command.user_id
                    && c.auth_session_id().is_none()
                    && *c.purpose() == MfaChallengePurpose::Enroll
                    && *c.method() == MfaMethod::Webauthn
                    && c.is_redeemable(now)
            })
            .ok_or_else(invalid)?;

        let state = challenge.state_json().cloned().ok_or_else(invalid)?;
        let registered = match self
            .webauthn
            .finish_registration(&command.attestation, &state)
        {
            Ok(registered) => registered,
            // Persist the failed attempt so a bad attestation cannot be
            // retried against the same challenge forever.
            Err(err) => {
                challenge.register_failed_attempt();
                uow.mfa_challenge_repository().update(&challenge).await?;
                uow.commit().await?;
                return Err(err);
            }
        };

        // Credential ids are globally unique: a passkey already bound to any
        // account (this one or another) must not be registered again.
        if uow
            .mfa_enrollment_repository()
            .find_by_credential_id(&registered.credential_id)
            .await?
            .is_some()
        {
            return Err(DomainError::Conflict(
                "this passkey is already registered".to_string(),
            ));
        }

        // Born confirmed: the registration ceremony is the proof of possession.
        let enrollment = MfaEnrollment::new_webauthn(
            command.tenant_id,
            command.user_id,
            registered.passkey,
            registered.credential_id,
            name,
        );
        challenge.consume(now);
        uow.mfa_challenge_repository().update(&challenge).await?;
        uow.mfa_enrollment_repository().add(&enrollment).await?;
        uow.commit().await?;

        Ok(Response {
            id: enrollment.id(),
            method: enrollment.method().as_str().to_string(),
            name: enrollment.name().map(str::to_string),
            created_at: enrollment.created_at(),
        })
    }
}

// Trim and drop empty labels; reject anything that would not fit the column.
// Shared with rename_my_mfa_enrollment, the other writer of enrollment names.
pub(crate) fn normalize_name(name: Option<String>) -> Result<Option<String>, DomainError> {
    match name.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(trimmed) if trimmed.chars().count() > MAX_NAME_LENGTH => {
            Err(DomainError::InvalidRequest(format!(
                "name too long (max {MAX_NAME_LENGTH} characters)"
            )))
        }
        Some(trimmed) => Ok(Some(trimmed.to_string())),
    }
}
