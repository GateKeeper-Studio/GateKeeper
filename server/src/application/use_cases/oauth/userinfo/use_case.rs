use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::{
        ports::token_verifier::TokenVerifier,
        use_cases::oauth::userinfo::{query::Query, response::Response},
    },
    domain::{
        errors::DomainError,
        repositories::{
            signing_key_repository::SigningKeyRepository, user_repository::UserRepository,
        },
    },
};

pub struct UseCase {
    user_repository: Arc<dyn UserRepository>,
    signing_key_repository: Arc<dyn SigningKeyRepository>,
    verifier: Arc<dyn TokenVerifier>,
}

impl UseCase {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        signing_key_repository: Arc<dyn SigningKeyRepository>,
        verifier: Arc<dyn TokenVerifier>,
    ) -> Self {
        Self {
            user_repository,
            signing_key_repository,
            verifier,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        // 1. Verify the bearer token against the tenant's published keys (by kid).
        let keys = self
            .signing_key_repository
            .find_published(query.tenant_id)
            .await?;
        if keys.is_empty() {
            return Err(DomainError::AccessDenied(
                "no published keys for tenant".to_string(),
            ));
        }
        let verified = self.verifier.verify(&query.access_token, &keys).await?;

        // 2. Resolve the subject to a user.
        let user_id = Uuid::parse_str(&verified.subject)
            .map_err(|_| DomainError::AccessDenied("invalid subject".to_string()))?;
        let user = self
            .user_repository
            .find_by_id_with_profile(user_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("user {user_id} not found")))?;

        // 3. Return only the claims the granted scopes allow.
        let has = |scope: &str| verified.scopes.iter().any(|s| s == scope);

        let mut response = Response {
            sub: user.id().to_string(),
            name: None,
            given_name: None,
            family_name: None,
            email: None,
            email_verified: None,
        };

        if has("profile") {
            if let Some(profile) = user.profile() {
                response.name = profile
                    .display_name()
                    .cloned()
                    .or_else(|| Some(format!("{} {}", profile.first_name(), profile.last_name())));
                response.given_name = Some(profile.first_name().to_string());
                response.family_name = Some(profile.last_name().to_string());
            }
        }

        if has("email") {
            response.email = Some(user.email().to_string());
            response.email_verified = Some(user.is_email_verified());
        }

        Ok(response)
    }
}
