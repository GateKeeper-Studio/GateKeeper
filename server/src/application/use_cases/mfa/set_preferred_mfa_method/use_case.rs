use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::mfa::set_preferred_mfa_method::command::Command,
    },
    domain::{entities::mfa_method::MfaMethod, errors::DomainError},
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

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let method = match command.method.as_deref() {
            None => None,
            Some(value) => Some(MfaMethod::parse(value).ok_or_else(|| {
                DomainError::InvalidRequest(format!("unknown MFA method: {value}"))
            })?),
        };

        // Email is implicit (always usable); totp/webauthn must be backed by a
        // confirmed enrollment, otherwise the next login would challenge a
        // factor the user cannot answer.
        if let Some(method) = method.filter(|m| *m != MfaMethod::Email) {
            let has_confirmed = uow
                .mfa_enrollment_repository()
                .find_confirmed_by_user_id(command.user_id)
                .await?
                .iter()
                .any(|e| e.tenant_id() == command.tenant_id && *e.method() == method);
            if !has_confirmed {
                return Err(DomainError::InvalidRequest(format!(
                    "no confirmed {} enrollment to set as default",
                    method.as_str()
                )));
            }
        }

        uow.user_repository()
            .set_preferred_2fa_method(command.user_id, method.map(|m| m.as_str()))
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
