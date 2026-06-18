use std::sync::Arc;

use crate::application::use_cases::legal_document::{
    get_legal_document::response::Response, reset_legal_document::command::Command,
};
use crate::domain::{
    errors::DomainError, repositories::legal_document_repository::LegalDocumentRepository,
};

// Removes a tenant's override so the built-in default is served again, and
// returns that default so the editor can refresh immediately.
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
        self.legal_document_repository
            .delete(command.tenant_id, command.document_type)
            .await?;

        Ok(Response::default_for(command.document_type))
    }
}
