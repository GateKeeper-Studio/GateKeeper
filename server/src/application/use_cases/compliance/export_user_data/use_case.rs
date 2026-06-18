use std::sync::Arc;

use crate::{
    application::use_cases::compliance::export_user_data::{
        query::Query,
        response::{ExportedConsent, ExportedUser, Response},
    },
    domain::{
        errors::DomainError,
        repositories::{
            consent_grant_repository::ConsentGrantRepository, group_repository::GroupRepository,
            role_repository::RoleRepository, user_repository::UserRepository,
        },
    },
};

// Read-only aggregation of a data subject's personal data across the system. Pure
// read — the audit event (`user.data_exported`) is emitted at the handler layer,
// where the actor/IP/UA request context lives.
pub struct UseCase {
    user_repository: Arc<dyn UserRepository>,
    consent_grant_repository: Arc<dyn ConsentGrantRepository>,
    role_repository: Arc<dyn RoleRepository>,
    group_repository: Arc<dyn GroupRepository>,
}

impl UseCase {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        consent_grant_repository: Arc<dyn ConsentGrantRepository>,
        role_repository: Arc<dyn RoleRepository>,
        group_repository: Arc<dyn GroupRepository>,
    ) -> Self {
        Self {
            user_repository,
            consent_grant_repository,
            role_repository,
            group_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let user = self
            .user_repository
            .find_by_id_with_profile(query.user_id)
            .await?
            .filter(|u| u.tenant_id() == query.tenant_id)
            .ok_or_else(|| {
                DomainError::UserNotFound(format!("user {} not found", query.user_id))
            })?;

        let profile = user.profile();
        let exported_user = ExportedUser {
            id: user.id(),
            tenant_id: user.tenant_id(),
            email: user.email().to_string(),
            email_verified: user.is_email_verified(),
            display_name: profile.and_then(|p| p.display_name().cloned()),
            first_name: profile.map(|p| p.first_name().to_string()),
            last_name: profile.map(|p| p.last_name().to_string()),
            is_active: user.is_active(),
            created_at: user.created_at().to_rfc3339(),
            updated_at: user.updated_at().map(|t| t.to_rfc3339()),
        };

        let consents = self
            .consent_grant_repository
            .find_by_user(query.user_id)
            .await?
            .into_iter()
            .map(|c| ExportedConsent {
                client_id: c.client_id(),
                scopes: c.scopes().to_vec(),
                revoked: c.is_revoked(),
                created_at: c.created_at().to_rfc3339(),
                updated_at: c.updated_at().map(|t| t.to_rfc3339()),
            })
            .collect();

        let roles = self
            .role_repository
            .find_by_user_id(query.tenant_id, query.user_id)
            .await?
            .iter()
            .map(|r| r.name().to_string())
            .collect();

        let groups = self
            .group_repository
            .find_by_user_id(query.tenant_id, query.user_id)
            .await?
            .iter()
            .map(|g| g.name().to_string())
            .collect();

        Ok(Response {
            generated_at: chrono::Utc::now().to_rfc3339(),
            user: exported_user,
            consents,
            roles,
            groups,
            policy_acceptances: Vec::new(), // populated once WI-04 ships
        })
    }
}
