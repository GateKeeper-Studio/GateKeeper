use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::email_template::{EmailTemplate, EmailTemplateType},
    errors::DomainError,
};

#[async_trait]
pub trait EmailTemplateRepository: Send + Sync {
    /// Returns the tenant's stored template, or `None` when it has not been
    /// customized (the caller then falls back to the built-in default).
    async fn find_by_tenant_and_type(
        &self,
        tenant_id: Uuid,
        template_type: EmailTemplateType,
    ) -> Result<Option<EmailTemplate>, DomainError>;

    /// Insert or replace the tenant's template (single-statement upsert).
    async fn upsert(&self, template: &EmailTemplate) -> Result<(), DomainError>;

    /// Remove the tenant's override so the default is served again. Idempotent.
    async fn delete(
        &self,
        tenant_id: Uuid,
        template_type: EmailTemplateType,
    ) -> Result<(), DomainError>;
}
