use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::{
    application::{
        shared::paged_content::PagedContent,
        use_cases::tenant_user::list_tenant_users::{query::Query, response::Response},
    },
    domain::{
        errors::DomainError,
        repositories::{
            social_identity_repository::SocialIdentityRepository,
            tenant_repository::TenantRepository, user_repository::UserRepository,
        },
    },
};

pub struct UseCase {
    user_repository: Arc<dyn UserRepository>,
    tenant_repository: Arc<dyn TenantRepository>,
    social_identity_repository: Arc<dyn SocialIdentityRepository>,
}

impl UseCase {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
        social_identity_repository: Arc<dyn SocialIdentityRepository>,
    ) -> Self {
        Self {
            user_repository,
            tenant_repository,
            social_identity_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<PagedContent<Response>, DomainError> {
        let tenant = self.tenant_repository.find_by_id(query.tenant_id).await?;

        if tenant.is_none() {
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                query.tenant_id
            )));
        }

        // Guard the pagination math: page must be >= 1 (offset = (page-1)*size) and
        // page_size is bounded so a request can't ask for an unbounded result set.
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

        let users = self
            .user_repository
            .find_by_tenant_id(query.tenant_id, page, page_size)
            .await?;
        let total = self
            .user_repository
            .quantity_of_users_by_tenant_id(query.tenant_id)
            .await?;

        // Batch-load the login methods for just this page of users (two queries,
        // not N+1) so each row can show "Password" / social-provider badges.
        let user_ids: Vec<_> = users.iter().map(|u| u.id()).collect();
        let with_password: HashSet<_> = self
            .user_repository
            .find_user_ids_with_password(&user_ids)
            .await?
            .into_iter()
            .collect();
        let mut providers_by_user: HashMap<_, Vec<String>> = HashMap::new();
        for (user_id, provider) in self
            .social_identity_repository
            .find_providers_for_users(&user_ids)
            .await?
        {
            providers_by_user
                .entry(user_id)
                .or_default()
                .push(provider.as_str().to_string());
        }

        Ok(PagedContent {
            items: users
                .into_iter()
                .map(|u| {
                    let profile = u.profile();
                    Response {
                        user_id: u.id(),
                        display_name: profile.and_then(|p| p.display_name().cloned()),
                        first_name: profile
                            .map(|p| p.first_name().to_string())
                            .unwrap_or_default(),
                        last_name: profile
                            .map(|p| p.last_name().to_string())
                            .unwrap_or_default(),
                        email: u.email().to_string(),
                        created_at: u.created_at().to_rfc3339(),
                        updated_at: u.updated_at().map(|t| t.to_rfc3339()),
                        has_password: with_password.contains(&u.id()),
                        providers: providers_by_user.remove(&u.id()).unwrap_or_default(),
                    }
                })
                .collect(),
            total,
            page,
            page_size,
        })
    }
}
