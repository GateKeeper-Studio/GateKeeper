use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::{external_identity_provider::ExternalProviderKind, social_identity::SocialIdentity},
    errors::DomainError,
};

#[async_trait]
pub trait SocialIdentityRepository: Send + Sync {
    // Recognise a returning social user by the provider's stable subject id.
    async fn find_by_provider_subject(
        &self,
        tenant_id: Uuid,
        provider: ExternalProviderKind,
        provider_subject: &str,
    ) -> Result<Option<SocialIdentity>, DomainError>;

    async fn add(&self, identity: &SocialIdentity) -> Result<(), DomainError>;

    // Linked providers for a batch of users (one query, no N+1). Used by the user
    // listing to render "Sign in with …" login-method badges.
    async fn find_providers_for_users(
        &self,
        user_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, ExternalProviderKind)>, DomainError>;
}
