// The platform-admin password check, shared by the two ways an admin signs in: the
// bearer-token API (`admin_auth::login`) and the console's cookie session
// (`admin_web_session::start`). One implementation, so the two can never drift in
// how they resist enumeration.
use crate::{
    application::ports::password_hasher::PasswordHasher,
    domain::{
        entities::admin_user::AdminUser, errors::DomainError,
        repositories::admin_user_repository::AdminUserRepository,
    },
};

// Returns the admin when the email and password match an active account.
//
// Every failure is the same generic `AuthenticationError`, so the response cannot
// tell "unknown email" from "wrong password" from "deactivated account". An unknown
// or inactive admin still pays one Argon2 verification (against a dummy hash), so the
// response time does not reveal which admin emails exist either.
pub async fn verify_admin_credentials(
    admins: &dyn AdminUserRepository,
    hasher: &dyn PasswordHasher,
    pepper: &str,
    email: &str,
    password: &str,
) -> Result<AdminUser, DomainError> {
    let invalid = || DomainError::AuthenticationError("invalid credentials".to_string());

    let admin = match admins.find_by_email(email).await? {
        Some(admin) if admin.is_active() => admin,
        _ => {
            hasher.verify_dummy(password).await;
            return Err(invalid());
        }
    };

    if !hasher.verify(admin.password_hash(), password, pepper).await {
        return Err(invalid());
    }
    Ok(admin)
}
