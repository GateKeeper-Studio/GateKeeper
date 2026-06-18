use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::legal_document::{LegalDocument, LegalDocumentType},
    errors::DomainError,
    repositories::legal_document_repository::LegalDocumentRepository,
};

/// Simple in-memory store for unit tests. Writes apply immediately (these use
/// cases don't go through the Unit of Work), so no draft/commit machinery.
pub struct InMemoryLegalDocumentRepository {
    documents: Mutex<HashMap<(Uuid, LegalDocumentType), LegalDocument>>,
}

impl InMemoryLegalDocumentRepository {
    pub fn new() -> Self {
        Self {
            documents: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryLegalDocumentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LegalDocumentRepository for InMemoryLegalDocumentRepository {
    async fn find_by_tenant_and_type(
        &self,
        tenant_id: Uuid,
        document_type: LegalDocumentType,
    ) -> Result<Option<LegalDocument>, DomainError> {
        let documents = self.documents.lock().unwrap();
        Ok(documents.get(&(tenant_id, document_type)).cloned())
    }

    async fn upsert(&self, document: &LegalDocument) -> Result<(), DomainError> {
        let mut documents = self.documents.lock().unwrap();
        documents.insert(
            (document.tenant_id(), document.document_type()),
            document.clone(),
        );
        Ok(())
    }

    async fn delete(
        &self,
        tenant_id: Uuid,
        document_type: LegalDocumentType,
    ) -> Result<(), DomainError> {
        let mut documents = self.documents.lock().unwrap();
        documents.remove(&(tenant_id, document_type));
        Ok(())
    }
}
