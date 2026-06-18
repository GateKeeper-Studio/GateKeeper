use std::sync::Arc;

use crate::application::use_cases::legal_document::{
    get_legal_document::response::Response, get_public_legal_document::query::Query,
};
use crate::domain::{
    errors::DomainError,
    repositories::{
        legal_document_repository::LegalDocumentRepository,
        oauth_client_repository::OAuthClientRepository,
    },
};

// Read-only public surface for the hosted login pages. Resolves the tenant from
// the client_id (the SPA never sees the tenant UUID), then serves that tenant's
// document or the built-in default. An unknown/inactive client falls back to the
// platform default so the legal pages always render.
pub struct UseCase {
    legal_document_repository: Arc<dyn LegalDocumentRepository>,
    oauth_client_repository: Arc<dyn OAuthClientRepository>,
}

impl UseCase {
    pub fn new(
        legal_document_repository: Arc<dyn LegalDocumentRepository>,
        oauth_client_repository: Arc<dyn OAuthClientRepository>,
    ) -> Self {
        Self {
            legal_document_repository,
            oauth_client_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let tenant_id = match self
            .oauth_client_repository
            .find_by_client_id_unscoped(&query.client_id)
            .await?
            .filter(|client| client.is_active())
        {
            Some(client) => client.tenant_id(),
            None => return Ok(Response::default_for(query.document_type)),
        };

        let stored = self
            .legal_document_repository
            .find_by_tenant_and_type(tenant_id, query.document_type)
            .await?;

        Ok(match stored {
            Some(document) => Response::customized(&document),
            None => Response::default_for(query.document_type),
        })
    }
}
