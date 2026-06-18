use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::mfa::{
            finish_webauthn_enrollment::use_case::normalize_name,
            rename_my_mfa_enrollment::{command::Command, response::Response},
        },
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

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let name = normalize_name(command.name)?;

        // Same NotFound whether the enrollment is missing or owned by someone
        // else — don't leak other users' enrollment ids.
        let mut enrollment = uow
            .mfa_enrollment_repository()
            .find_by_id(command.tenant_id, command.enrollment_id)
            .await?
            .filter(|e| e.user_id() == command.user_id)
            .ok_or_else(|| {
                DomainError::MfaEnrollmentNotFound("enrollment not found".to_string())
            })?;

        enrollment.rename(name);
        uow.mfa_enrollment_repository().update(&enrollment).await?;
        uow.commit().await?;

        Ok(Response {
            id: enrollment.id(),
            name: enrollment.name().map(str::to_string),
        })
    }
}
