use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::policy_acceptance::PolicyAcceptance, errors::DomainError};

#[async_trait]
pub trait PolicyAcceptanceRepository: Send + Sync {
    // Idempotent on (user_id, document_type, document_version): re-accepting the same
    // version is a no-op rather than an error.
    async fn record(&self, acceptance: &PolicyAcceptance) -> Result<(), DomainError>;

    async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<PolicyAcceptance>, DomainError>;

    // The most recent acceptance of a given document type, used to decide whether the
    // currently-required version has been accepted.
    async fn latest_for(
        &self,
        user_id: Uuid,
        document_type: &str,
    ) -> Result<Option<PolicyAcceptance>, DomainError>;
}
