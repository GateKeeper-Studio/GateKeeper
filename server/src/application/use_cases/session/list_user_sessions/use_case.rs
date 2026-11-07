use std::{collections::HashMap, sync::Arc};

use chrono::Utc;

use crate::{
    application::use_cases::session::{
        list_user_sessions::query::Query, session_view::SessionView,
    },
    domain::{
        errors::DomainError,
        repositories::{
            oauth_client_repository::OAuthClientRepository,
            refresh_token_repository::RefreshTokenRepository, user_repository::UserRepository,
        },
    },
};

// The user's signed-in sessions: one row per active refresh-token family.
pub struct UseCase {
    refresh_token_repository: Arc<dyn RefreshTokenRepository>,
    oauth_client_repository: Arc<dyn OAuthClientRepository>,
    user_repository: Arc<dyn UserRepository>,
}

impl UseCase {
    pub fn new(
        refresh_token_repository: Arc<dyn RefreshTokenRepository>,
        oauth_client_repository: Arc<dyn OAuthClientRepository>,
        user_repository: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            refresh_token_repository,
            oauth_client_repository,
            user_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Vec<SessionView>, DomainError> {
        // The user must belong to this tenant — no listing another tenant's sessions
        // through a mismatched path.
        self.user_repository
            .find_by_id(query.user_id)
            .await?
            .filter(|u| u.tenant_id() == query.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("User with id {} not found", query.user_id))
            })?;

        let families = self
            .refresh_token_repository
            .find_active_families_by_user(query.tenant_id, query.user_id, Utc::now())
            .await?;

        // A handful of apps per user at most: resolve each one once.
        let mut clients = HashMap::new();
        let mut views = Vec::with_capacity(families.len());
        for family in families {
            if !clients.contains_key(&family.client_id) {
                let client = self
                    .oauth_client_repository
                    .find_by_id(query.tenant_id, family.client_id)
                    .await?;
                clients.insert(family.client_id, client);
            }
            let client = clients.get(&family.client_id).and_then(Option::as_ref);

            views.push(SessionView {
                id: family.family_id,
                client_id: client
                    .map(|c| c.client_id().to_string())
                    .unwrap_or_default(),
                client_name: client.map(|c| c.name().to_string()),
                scopes: family.scopes,
                signed_in_at: family.signed_in_at,
                last_active_at: family.last_used_at,
                expires_at: family.expires_at,
            });
        }
        Ok(views)
    }
}
