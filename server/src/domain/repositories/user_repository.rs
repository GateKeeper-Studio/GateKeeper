use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{entities::user::User, errors::DomainError};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, DomainError>;
    async fn find_by_id_with_profile(&self, user_id: Uuid) -> Result<Option<User>, DomainError>;
    async fn find_by_id_with_credentials(&self, user_id: Uuid)
    -> Result<Option<User>, DomainError>;

    async fn find_by_email(
        &self,
        tenant_id: Uuid,
        email: &str,
    ) -> Result<Option<User>, DomainError>;
    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<User>, DomainError>;

    async fn quantity_of_users_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError>;
    async fn is_user_with_email_exists(
        &self,
        tenant_id: Uuid,
        email: &str,
    ) -> Result<bool, DomainError>;

    // Which of the given users have a password credential. Used by the user
    // listing to show login-method badges without hydrating every credential row.
    async fn find_user_ids_with_password(
        &self,
        user_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError>;

    async fn delete_by_id(&self, user_id: Uuid) -> Result<(), DomainError>;

    // Right to erasure (WI-06): scrub PII in-place (email → tombstone, names cleared),
    // deactivate the account, set `anonymized_at`, and remove credentials — while
    // keeping the row so audit/referential history survives. Refuses with AccessDenied
    // when the user is under a legal hold; UserNotFound when the id does not exist.
    async fn anonymize(&self, user_id: Uuid) -> Result<(), DomainError>;

    async fn add(&self, user: &User) -> Result<(), DomainError>;
    // Saves the user. A copy of the credentials read earlier can never extend or
    // revive a temporary password: with the same hash, the earlier of the stored and
    // the saved expiry wins.
    async fn update(&self, user: &User) -> Result<(), DomainError>;

    // Spends the user's temporary password (an admin reset's): from `now` on it no
    // longer signs in. One conditional statement, so of two sign-ins racing with the
    // same password only one gets `true`; `false` when it was already used, expired,
    // or the user has no such password.
    async fn burn_temporary_password(
        &self,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, DomainError>;

    // The user's default second factor ("email" | "totp" | "webauthn"). Kept out
    // of the User aggregate (like legal_hold/anonymized_at): only the MFA use
    // cases read or write it, so it is fetched on demand rather than hydrated on
    // every user load.
    async fn get_preferred_2fa_method(&self, user_id: Uuid) -> Result<Option<String>, DomainError>;
    async fn set_preferred_2fa_method(
        &self,
        user_id: Uuid,
        method: Option<&str>,
    ) -> Result<(), DomainError>;
}
