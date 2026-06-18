use std::sync::Arc;

use crate::application::use_cases::legal_document::get_legal_document::{
    query::Query, response::Response,
};
use crate::domain::{
    errors::DomainError, repositories::legal_document_repository::LegalDocumentRepository,
};

// Read-only: returns the tenant's document or the built-in default. Takes the
// repository arc directly per the read-use-case convention.
pub struct UseCase {
    legal_document_repository: Arc<dyn LegalDocumentRepository>,
}

impl UseCase {
    pub fn new(legal_document_repository: Arc<dyn LegalDocumentRepository>) -> Self {
        Self {
            legal_document_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let stored = self
            .legal_document_repository
            .find_by_tenant_and_type(query.tenant_id, query.document_type)
            .await?;

        Ok(match stored {
            Some(document) => Response::customized(&document),
            None => Response::default_for(query.document_type),
        })
    }
}
