use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{entities::consent_grant::ConsentGrant, errors::DomainError};

#[async_trait]
pub trait ConsentGrantRepository: Send + Sync {
    async fn find_by_user_and_client(
        &self,
        user_id: Uuid,
        client_id: Uuid,
    ) -> Result<Option<ConsentGrant>, DomainError>;
    async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<ConsentGrant>, DomainError>; // "my apps" screen
    async fn upsert(&self, grant: &ConsentGrant) -> Result<(), DomainError>; // ON CONFLICT (user_id, client_id)
    async fn revoke(&self, user_id: Uuid, client_id: Uuid) -> Result<(), DomainError>;
    // Remove all consent grants for a user — used by erasure (WI-06).
    async fn delete_by_user_id(&self, user_id: Uuid) -> Result<(), DomainError>;
}
