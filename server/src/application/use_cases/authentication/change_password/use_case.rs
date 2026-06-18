use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::authentication::change_password::command::Command,
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

    // One transaction: validate the one-time change code, rotate the password,
    // clear the should_change_password flag, burn the code, and revoke existing
    // sessions together.
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

        // 2. Generic error for every validation failure so an attacker cannot tell
        //    why a change attempt failed (unknown user, expired, consumed, wrong code).
        let invalid = || {
            DomainError::PasswordChangeError("invalid or expired password change code".to_string())
        };

        let user_id = Uuid::parse_str(&command.user_id).map_err(|_| invalid())?;

        // 3. Resolve the user's most recent active change token and enforce all guards
        //    before comparing the secret.
        let mut token = uow
            .password_change_token_repository()
            .find_active_by_user_id(user_id)
            .await?
            .filter(|t| t.tenant_id() == tenant_id && t.is_redeemable(Utc::now()))
            .ok_or_else(invalid)?;

        // 4. Hash comparison of the high-entropy secret (constant-time on the hash).
        if sha256_hex(&command.change_password_code) != token.token_hash() {
            return Err(invalid());
        }

        // 5. Load the user (with credentials) and the tenant secret used to pepper
        //    the hash.
        let mut user = uow
            .user_repository()
            .find_by_id_with_credentials(user_id)
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

        // 7. Rotate the password and clear the forced-change flag so the user can now
        //    complete authorization.
        user.update_credentials(UserCredentials::new(
            user.id(),
            hashed_password,
            "argon2".to_string(),
            false,
        ));
        uow.user_repository().update(&user).await?;

        // 8. Burn the change code (single-use).
        token.consume(Utc::now());
        uow.password_change_token_repository()
            .update(&token)
            .await?;

        // 9. Lock out anyone who had access under the temporary password: revoke every
        //    live refresh token the user holds across all clients.
        uow.refresh_token_repository()
            .revoke_by_user(tenant_id, user_id)
            .await?;

        uow.commit().await?;

        Ok(())
    }
}
