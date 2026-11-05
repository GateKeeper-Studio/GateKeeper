use std::sync::Arc;

use chrono::Utc;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::authentication::reset_password::command::Command,
    },
    domain::{entities::user_credentials::UserCredentials, errors::DomainError},
    utils::{oauth_crypto::sha256_hex, password_hasher::hash_password},
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

    // One transaction: validate the reset token, rotate the password, verify the
    // email, burn the token, and revoke existing sessions together.
    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // 1. Resolve the client → tenant; never trust a caller-supplied tenant.
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;
        let tenant_id = client.tenant_id();

        // 1b. An external-login-only client mints no reset tokens; refuse any that
        //     predate the flag being turned on.
        if !client.allows_password_login() {
            return Err(DomainError::AccessDenied(
                "this application signs in with an external provider only".to_string(),
            ));
        }

        // 2. Generic error for every validation failure so an attacker cannot tell
        //    why a reset attempt failed (unknown id, expired, consumed, wrong token).
        let invalid = || {
            DomainError::PasswordResetError("invalid or expired password reset token".to_string())
        };

        let reset_id = Uuid::parse_str(&command.password_reset_id).map_err(|_| invalid())?;

        // 3. Look the token up by its id (carried in the link), then enforce all
        //    guards before comparing the secret.
        let mut token = uow
            .password_reset_token_repository()
            .find_by_id(reset_id)
            .await?
            .filter(|t| t.tenant_id() == tenant_id && t.is_redeemable(Utc::now()))
            .ok_or_else(invalid)?;

        // 4. Constant-time hash comparison of the high-entropy secret.
        let submitted = sha256_hex(&command.password_reset_token);
        if submitted
            .as_bytes()
            .ct_eq(token.token_hash().as_bytes())
            .unwrap_u8()
            != 1
        {
            return Err(invalid());
        }

        // 5. Load the user (with credentials) and the tenant secret used to pepper
        //    the hash.
        let mut user = uow
            .user_repository()
            .find_by_id_with_credentials(token.user_id())
            .await?
            .filter(|u| u.is_active() && u.tenant_id() == tenant_id)
            .ok_or_else(invalid)?;

        let tenant = uow
            .tenant_repository()
            .find_by_id(tenant_id)
            .await?
            .filter(|t| t.is_active())
            .ok_or_else(invalid)?;

        // 6. Argon2 is intentionally CPU/memory heavy; run it off the async worker.
        let password = command.new_password.clone();
        let secret = tenant.password_hash_secret().to_string();
        let hashed_password =
            tokio::task::spawn_blocking(move || hash_password(&password, &secret))
                .await
                .map_err(|_| {
                    DomainError::PasswordHashingError("Failed to hash password".to_string())
                })?
                .map_err(|_| {
                    DomainError::PasswordHashingError("Failed to hash password".to_string())
                })?;

        user.update_credentials(UserCredentials::new(
            user.id(),
            hashed_password,
            "argon2".to_string(),
            false,
        ));
        // Possessing the emailed link proves control of the address, so a reset also
        // verifies the email — this unblocks users locked out by an unverified one.
        user.verify_email();
        uow.user_repository().update(&user).await?;

        // 7. Burn the reset token (single-use).
        token.consume(Utc::now());
        uow.password_reset_token_repository().update(&token).await?;

        // 8. Lock out anyone who had access under the old password: revoke every
        //    live refresh token the user holds across all clients.
        uow.refresh_token_repository()
            .revoke_by_user(tenant_id, user.id())
            .await?;

        uow.commit().await?;

        Ok(())
    }
}
