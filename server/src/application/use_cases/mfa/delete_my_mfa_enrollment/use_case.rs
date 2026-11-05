use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::mfa::delete_my_mfa_enrollment::command::Command,
    },
    domain::errors::DomainError,
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    // Deleting is always safe: email OTP is implicit, so the user can never
    // lock themselves out of MFA entirely.
    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // Same NotFound whether the enrollment is missing or owned by someone
        // else — don't leak other users' enrollment ids.
        let enrollment = uow
            .mfa_enrollment_repository()
            .find_by_id(command.tenant_id, command.enrollment_id)
            .await?
            .filter(|e| e.user_id() == command.user_id)
            .ok_or_else(|| DomainError::NotFound("enrollment not found".to_string()))?;

        uow.mfa_enrollment_repository()
            .delete_by_id(command.tenant_id, command.enrollment_id)
            .await?;

        // If this was the last confirmed enrollment of the preferred method,
        // clear the preference so login falls back to its priority chain
        // (webauthn > totp > email) instead of challenging a gone factor.
        let preferred = uow
            .user_repository()
            .get_preferred_2fa_method(command.user_id)
            .await?;
        if preferred.as_deref() == Some(enrollment.method().as_str()) {
            let method_still_usable = uow
                .mfa_enrollment_repository()
                .find_confirmed_by_user_id(command.user_id)
                .await?
                .iter()
                .any(|e| e.tenant_id() == command.tenant_id && e.method() == enrollment.method());
            if !method_still_usable {
                uow.user_repository()
                    .set_preferred_2fa_method(command.user_id, None)
                    .await?;
            }
        }

        uow.commit().await?;
        Ok(())
    }
}
