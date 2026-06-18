use async_trait::async_trait;
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

    async fn delete_by_id(&self, user_id: Uuid) -> Result<(), DomainError>;

    // Right to erasure (WI-06): scrub PII in-place (email → tombstone, names cleared),
    // deactivate the account, set `anonymized_at`, and remove credentials — while
    // keeping the row so audit/referential history survives. Refuses with AccessDenied
    // when the user is under a legal hold; UserNotFound when the id does not exist.
    async fn anonymize(&self, user_id: Uuid) -> Result<(), DomainError>;

    async fn add(&self, user: &User) -> Result<(), DomainError>;
    async fn update(&self, user: &User) -> Result<(), DomainError>;

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
