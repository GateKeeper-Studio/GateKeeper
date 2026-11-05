use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::application::use_cases::legal_document::{
    get_legal_document::response::Response, update_legal_document::command::Command,
};
use crate::domain::repositories::tenant_repository::TenantRepository;
use crate::domain::{
    entities::legal_document::LegalDocument, errors::DomainError,
    repositories::legal_document_repository::LegalDocumentRepository,
};

// Stores (or replaces) a tenant's document. A single upsert is atomic on its
// own, so this takes the repository directly rather than a Unit of Work. The
// tenant FK on `legal_documents` rejects writes for a non-existent tenant.
pub struct UseCase {
    tenant_repository: Arc<dyn TenantRepository>,
    legal_document_repository: Arc<dyn LegalDocumentRepository>,
}

impl UseCase {
    pub fn new(
        legal_document_repository: Arc<dyn LegalDocumentRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            legal_document_repository,
            tenant_repository,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        actor
            .authorize_tenant_admin_for(self.tenant_repository.as_ref(), command.tenant_id)
            .await?;
        let document = LegalDocument::new(
            command.tenant_id,
            command.document_type,
            command.markdown_content,
        );

        self.legal_document_repository.upsert(&document).await?;

        Ok(Response::customized(&document))
    }
}
