#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::consent_grant::list_user_consents::{
            query::Query, use_case::UseCase,
        },
        domain::{
            entities::{
                consent_grant::ConsentGrant,
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            user_factory::generate_user,
        },
    };

    fn make_client(tenant_id: Uuid, name: &str) -> OAuthClient {
        OAuthClient::new(
            tenant_id,
            Uuid::now_v7(),
            None,
            ClientType::Public,
            name.to_string(),
            vec!["https://app.example.com/cb".to_string()],
            vec![],
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            MfaPolicy::default(),
        )
    }

    #[tokio::test]
    async fn lists_active_consents_with_client_name() {
        let tenant_id = Uuid::now_v7();
        let user = generate_user(tenant_id);
        let user_id = user.id();

        let users = Arc::new(InMemoryUserRepository::new());
        users.memory.data.lock().unwrap().insert(user_id, user);

        let client = make_client(tenant_id, "alpha");
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let consents = Arc::new(InMemoryConsentGrantRepository::new());
        let active = ConsentGrant::new(tenant_id, user_id, client.id(), vec!["openid".to_string()]);
        let mut revoked = ConsentGrant::new(
            tenant_id,
            user_id,
            Uuid::now_v7(),
            vec!["email".to_string()],
        );
        revoked.revoke();
        {
            let mut data = consents.memory.data.lock().unwrap();
            data.insert(active.id(), active);
            data.insert(revoked.id(), revoked);
        }

        let use_case = UseCase::new(consents, clients, users);
        let views = use_case
            .execute(Query { tenant_id, user_id })
            .await
            .unwrap();

        // Only the active grant is returned, enriched with the client name.
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].client_id, client.id());
        assert_eq!(views[0].client_name.as_deref(), Some("alpha"));
    }

    #[tokio::test]
    async fn fails_when_user_not_in_tenant() {
        let tenant_id = Uuid::now_v7();
        let users = Arc::new(InMemoryUserRepository::new());
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        let consents = Arc::new(InMemoryConsentGrantRepository::new());

        let use_case = UseCase::new(consents, clients, users);
        let err = use_case
            .execute(Query {
                tenant_id,
                user_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::UserNotFound(_)));
    }
}
