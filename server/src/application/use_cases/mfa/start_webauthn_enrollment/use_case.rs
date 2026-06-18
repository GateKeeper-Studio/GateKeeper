use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::{
    application::{
        ports::{unit_of_work::UnitOfWorkFactory, webauthn::WebauthnService},
        use_cases::mfa::start_webauthn_enrollment::{command::Command, response::Response},
    },
    domain::{
        entities::{
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_method::MfaMethod,
        },
        errors::DomainError,
    },
};

const MFA_CHALLENGE_TTL_MINUTES: i64 = 5;

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

        let user = uow
            .user_repository()
            .find_by_id_with_profile(command.user_id)
            .await?
            .filter(|u| u.tenant_id() == command.tenant_id && u.is_active())
            .ok_or_else(|| DomainError::UserNotFound("user not found".to_string()))?;
        let display_name = user
            .profile()
            .and_then(|p| p.display_name().cloned())
            .unwrap_or_else(|| user.email().to_string());

        // Exclude already-registered credentials so the browser refuses to
        // re-register the same authenticator.
        let exclude_credential_ids: Vec<String> = uow
            .mfa_enrollment_repository()
            .find_confirmed_by_user_id(command.user_id)
            .await?
            .iter()
            .filter(|e| e.tenant_id() == command.tenant_id && *e.method() == MfaMethod::Webauthn)
            .filter_map(|e| e.credential_id().map(str::to_string))
            .collect();

        let ceremony = self.webauthn.start_registration(
            command.user_id,
            user.email(),
            &display_name,
            &exclude_credential_ids,
        )?;

        // Burn any stale enrollment ceremony before minting a new one.
        uow.mfa_challenge_repository()
            .invalidate_active_by_user(
                command.user_id,
                &MfaChallengePurpose::Enroll,
                &MfaMethod::Webauthn,
            )
            .await?;

        // No auth session: the enrollment is authorized by the bearer token,
        // not by a pending login.
        let challenge = MfaChallenge::new_webauthn(
            command.tenant_id,
            command.user_id,
            None,
            MfaChallengePurpose::Enroll,
            ceremony.state,
            now + Duration::minutes(MFA_CHALLENGE_TTL_MINUTES),
        );
        let mfa_id = challenge.id().to_string();
        uow.mfa_challenge_repository().add(&challenge).await?;
        uow.commit().await?;

        Ok(Response {
            mfa_id,
            options: ceremony.options,
        })
    }
}
