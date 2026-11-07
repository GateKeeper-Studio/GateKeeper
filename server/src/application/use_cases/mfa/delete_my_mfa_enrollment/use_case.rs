use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::mfa::delete_my_mfa_enrollment::command::Command,
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

    // Deleting a second factor is safe: email OTP is implicit, so a user with a
    // password can never lock themselves out of MFA entirely.
    //
    // The exception is passwordless login: for an account with no password a passkey
    // is the primary factor. Its LAST passkey may only go while another way in
    // remains (a linked social identity); otherwise the user must first set a
    // password — through forgot-password, the recovery path — or add another passkey.
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

        if *enrollment.method() == MfaMethod::Webauthn {
            // No user row means there is no account left to lock out.
            let has_password = uow
                .user_repository()
                .find_by_id_with_credentials(command.user_id)
                .await?
                .is_none_or(|u| u.credentials().is_some());
            if !has_password {
                let another_passkey = uow
                    .mfa_enrollment_repository()
                    .find_confirmed_by_user_id(command.user_id)
                    .await?
                    .iter()
                    .any(|e| *e.method() == MfaMethod::Webauthn && e.id() != enrollment.id());
                let has_social_identity = !uow
                    .social_identity_repository()
                    .find_providers_for_users(&[command.user_id])
                    .await?
                    .is_empty();
                if !another_passkey && !has_social_identity {
                    return Err(DomainError::Conflict(
                        "cannot remove the last passkey of an account without a password; \
                         set a password (forgot password) or add another passkey first"
                            .to_string(),
                    ));
                }
            }
        }

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
