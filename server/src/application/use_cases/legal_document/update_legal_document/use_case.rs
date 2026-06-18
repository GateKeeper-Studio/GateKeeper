use std::sync::Arc;

use crate::application::use_cases::legal_document::{
    get_legal_document::response::Response, update_legal_document::command::Command,
};
use crate::domain::{
    entities::legal_document::LegalDocument, errors::DomainError,
    repositories::legal_document_repository::LegalDocumentRepository,
};

// Stores (or replaces) a tenant's document. A single upsert is atomic on its
// own, so this takes the repository directly rather than a Unit of Work. The
// tenant FK on `legal_documents` rejects writes for a non-existent tenant.
pub struct UseCase {
    legal_document_repository: Arc<dyn LegalDocumentRepository>,
}

impl UseCase {
    pub fn new(legal_document_repository: Arc<dyn LegalDocumentRepository>) -> Self {
        Self {
            legal_document_repository,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let document = LegalDocument::new(
            command.tenant_id,
            command.document_type,
            command.markdown_content,
        );

        self.legal_document_repository.upsert(&document).await?;

        Ok(Response::customized(&document))
    }
}
