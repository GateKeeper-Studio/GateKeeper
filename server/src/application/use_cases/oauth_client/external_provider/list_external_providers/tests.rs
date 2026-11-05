#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::external_provider::list_external_providers::{
            query::Query, use_case::UseCase,
        },
        domain::entities::{
            external_identity_provider::{ExternalIdentityProvider, ExternalProviderKind},
            oauth_client::{ClientType, MfaPolicy, OAuthClient},
        },
        testing::in_memory_repositories::{
            in_memory_external_identity_provider_repository::InMemoryExternalIdentityProviderRepository,
            in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
        },
    };

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::reconstruct(
            Uuid::now_v7(),
            tenant_id,
            Uuid::now_v7(),
            None,
            ClientType::Public,
            "App".to_string(),
            vec!["https://app.example.com/cb".to_string()],
            vec![],
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            None,
            None,
            None,
            None,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
            true,
            chrono::Utc::now(),
            None,
        )
    }

    #[tokio::test]
    async fn lists_configured_providers_without_secrets() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let providers = Arc::new(InMemoryExternalIdentityProviderRepository::new());
        let google = ExternalIdentityProvider::new(
            client.id(),
            tenant_id,
            ExternalProviderKind::Google,
            "g-id".to_string(),
            "enc:g-secret".to_string(),
            "https://idp.example.com/cb/google".to_string(),
            true,
        );
        {
            let mut data = providers.memory.data.lock().unwrap();
            data.insert(google.id(), google.clone());
        }

        let use_case = UseCase::new(Arc::clone(&providers) as _, Arc::clone(&clients) as _);
        let views = use_case
            .execute(Query {
                tenant_id,
                oauth_client_id: client.id(),
            })
            .await
            .unwrap();

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].provider, "google");
        assert_eq!(views[0].provider_client_id, "g-id");
    }
}
