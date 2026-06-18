#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::use_cases::consent_grant::revoke_consent::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{consent_grant::ConsentGrant, refresh_token::RefreshToken},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
                in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            user_factory::generate_user,
        },
    };

    #[tokio::test]
    async fn revokes_consent_and_kills_live_refresh_tokens() {
        let tenant_id = Uuid::now_v7();
        let user = generate_user(tenant_id);
        let user_id = user.id();
        let client_id = Uuid::now_v7();

        let users = Arc::new(InMemoryUserRepository::new());
        users.memory.data.lock().unwrap().insert(user_id, user);

        let consents = Arc::new(InMemoryConsentGrantRepository::new());
        let grant = ConsentGrant::new(tenant_id, user_id, client_id, vec!["openid".to_string()]);
        consents
            .memory
            .data
            .lock()
            .unwrap()
            .insert(grant.id(), grant);

        let refresh = Arc::new(InMemoryRefreshTokenRepository::new());
        let token = RefreshToken::new(
            tenant_id,
            "hash".to_string(),
            client_id,
            user_id,
            vec!["openid".to_string()],
            Utc::now() + Duration::days(30),
        );
        refresh
            .memory
            .data
            .lock()
            .unwrap()
            .insert(token.id(), token);

        let factory =
            InMemoryUnitOfWorkFactory::new(users, Arc::new(InMemoryTenantRepository::new()))
                .with_consent_grant_repository(consents.clone())
                .with_refresh_token_repository(refresh.clone());

        let use_case = UseCase::new(Arc::new(factory));
        use_case
            .execute(Command {
                tenant_id,
                user_id,
                client_id,
            })
            .await
            .unwrap();

        // Consent grant is now revoked.
        let grants = consents.memory.data.lock().unwrap();
        assert!(grants.values().next().unwrap().is_revoked());

        // The live refresh token for that (user, client) is revoked.
        let tokens = refresh.memory.data.lock().unwrap();
        assert!(tokens.values().next().unwrap().revoked_at().is_some());
    }

    #[tokio::test]
    async fn fails_when_user_not_in_tenant() {
        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        );
        let use_case = UseCase::new(Arc::new(factory));

        let err = use_case
            .execute(Command {
                tenant_id: Uuid::now_v7(),
                user_id: Uuid::now_v7(),
                client_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::UserNotFound(_)));
    }
}
