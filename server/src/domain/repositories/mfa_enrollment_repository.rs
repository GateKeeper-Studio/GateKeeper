use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::mfa_enrollment::MfaEnrollment, errors::DomainError};

#[async_trait]
pub trait MfaEnrollmentRepository: Send + Sync {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<MfaEnrollment>, DomainError>;
    // All enrollments, confirmed or pending (admin/self-service listing).
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<MfaEnrollment>, DomainError>;
    // Only factors usable for a login challenge.
    async fn find_confirmed_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<MfaEnrollment>, DomainError>;
    // The user's single TOTP row (pending or confirmed), if any.
    async fn find_totp_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<MfaEnrollment>, DomainError>;
    // WebAuthn assertion result → enrollment, via the globally-unique credential id.
    async fn find_by_credential_id(
        &self,
        credential_id: &str,
    ) -> Result<Option<MfaEnrollment>, DomainError>;

    async fn add(&self, enrollment: &MfaEnrollment) -> Result<(), DomainError>;
    async fn update(&self, enrollment: &MfaEnrollment) -> Result<(), DomainError>;
    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError>;
}
