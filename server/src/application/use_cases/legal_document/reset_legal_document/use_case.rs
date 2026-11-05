use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::application::use_cases::legal_document::{
    get_legal_document::response::Response, reset_legal_document::command::Command,
};
use crate::domain::repositories::tenant_repository::TenantRepository;
use crate::domain::{
    errors::DomainError, repositories::legal_document_repository::LegalDocumentRepository,
};

// Removes a tenant's override so the built-in default is served again, and
// returns that default so the editor can refresh immediately.
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
        self.legal_document_repository
            .delete(command.tenant_id, command.document_type)
            .await?;

        Ok(Response::default_for(command.document_type))
    }
}
