use std::sync::Arc;

use crate::{
    application::use_cases::consent_grant::{
        consent_view::ConsentView, list_user_consents::query::Query,
    },
    domain::{
        errors::DomainError,
        repositories::{
            consent_grant_repository::ConsentGrantRepository,
            oauth_client_repository::OAuthClientRepository, user_repository::UserRepository,
        },
    },
};

pub struct UseCase {
    consent_grant_repository: Arc<dyn ConsentGrantRepository>,
    oauth_client_repository: Arc<dyn OAuthClientRepository>,
    user_repository: Arc<dyn UserRepository>,
}

impl UseCase {
    pub fn new(
        consent_grant_repository: Arc<dyn ConsentGrantRepository>,
        oauth_client_repository: Arc<dyn OAuthClientRepository>,
        user_repository: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            consent_grant_repository,
            oauth_client_repository,
            user_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Vec<ConsentView>, DomainError> {
        // The user must exist and belong to this tenant — prevents listing another
        // tenant's consents through a mismatched path.
        self.user_repository
            .find_by_id(query.user_id)
            .await?
            .filter(|u| u.tenant_id() == query.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("User with id {} not found", query.user_id))
            })?;

        let grants = self
            .consent_grant_repository
            .find_by_user(query.user_id)
            .await?;

        let mut views = Vec::new();
        for grant in grants {
            // Scope to the tenant and show only currently-active (non-revoked) consents.
            if grant.tenant_id() != query.tenant_id || grant.is_revoked() {
                continue;
            }

            let client_name = self
                .oauth_client_repository
                .find_by_id(query.tenant_id, grant.client_id())
                .await?
                .map(|c| c.name().to_string());

            views.push(ConsentView {
                client_id: grant.client_id(),
                client_name,
                scopes: grant.scopes().to_vec(),
                created_at: grant.created_at().to_rfc3339(),
                updated_at: grant.updated_at().map(|t| t.to_rfc3339()),
            });
        }

        Ok(views)
    }
}
