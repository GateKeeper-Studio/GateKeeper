use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::external_identity_provider::ExternalProviderKind;

// Links a local user to an external provider account. On a returning social
// login the user is recognised by (tenant_id, provider, provider_subject) — the
// provider's stable subject id — without depending on the email alone.
#[derive(Debug, Clone)]
pub struct SocialIdentity {
    id: Uuid,
    user_id: Uuid,
    tenant_id: Uuid,
    provider: ExternalProviderKind,
    provider_subject: String,
    email: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl SocialIdentity {
    pub fn new(
        user_id: Uuid,
        tenant_id: Uuid,
        provider: ExternalProviderKind,
        provider_subject: String,
        email: Option<String>,
    ) -> Self {
        SocialIdentity {
            id: Uuid::now_v7(),
            user_id,
            tenant_id,
            provider,
            provider_subject,
            email,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        user_id: Uuid,
        tenant_id: Uuid,
        provider: ExternalProviderKind,
        provider_subject: String,
        email: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        SocialIdentity {
            id,
            user_id,
            tenant_id,
            provider,
            provider_subject,
            email,
            created_at,
            updated_at,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn provider(&self) -> ExternalProviderKind {
        self.provider
    }
    pub fn provider_subject(&self) -> &str {
        &self.provider_subject
    }
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}
