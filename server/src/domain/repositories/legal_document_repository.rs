use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::legal_document::{LegalDocument, LegalDocumentType},
    errors::DomainError,
};

#[async_trait]
pub trait LegalDocumentRepository: Send + Sync {
    /// Returns the tenant's stored document, or `None` when it has not been
    /// customized (the caller then falls back to the built-in default).
    async fn find_by_tenant_and_type(
        &self,
        tenant_id: Uuid,
        document_type: LegalDocumentType,
    ) -> Result<Option<LegalDocument>, DomainError>;

    /// Insert or replace the tenant's document (single-statement upsert).
    async fn upsert(&self, document: &LegalDocument) -> Result<(), DomainError>;

    /// Remove the tenant's override so the default is served again. Idempotent.
    async fn delete(
        &self,
        tenant_id: Uuid,
        document_type: LegalDocumentType,
    ) -> Result<(), DomainError>;
}
