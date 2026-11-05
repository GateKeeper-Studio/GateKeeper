#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::external_provider::create_external_provider::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::oauth_client::{ClientType, MfaPolicy, OAuthClient},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_external_identity_provider_repository::InMemoryExternalIdentityProviderRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            stub_secret_cipher::StubSecretCipher,
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

    fn factory_with_client(
        client: &OAuthClient,
        providers: Arc<InMemoryExternalIdentityProviderRepository>,
    ) -> Arc<InMemoryUnitOfWorkFactory> {
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_oauth_client_repository(clients)
            .with_external_identity_provider_repository(providers),
        )
    }

    #[tokio::test]
    async fn creates_provider_and_encrypts_secret() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let providers = Arc::new(InMemoryExternalIdentityProviderRepository::new());
        let factory = factory_with_client(&client, Arc::clone(&providers));
        let use_case = UseCase::new(factory, Arc::new(StubSecretCipher));

        let view = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    oauth_client_id: client.id(),
                    provider: "google".to_string(),
                    client_id: "google-client".to_string(),
                    client_secret: "supersecret".to_string(),
                    redirect_uri: "https://idp.example.com/v1/auth/oauth-provider/google/callback"
                        .to_string(),
                    is_active: true,
                },
            )
            .await
            .unwrap();

        assert_eq!(view.provider, "google");
        assert_eq!(view.provider_client_id, "google-client");

        // Committed, and the secret is stored encrypted (never in clear).
        let stored = providers.memory.data.lock().unwrap();
        let saved = stored.values().next().unwrap();
        assert_ne!(saved.provider_client_secret_encrypted(), "supersecret");
        assert_eq!(saved.provider_client_secret_encrypted(), "enc:supersecret");
    }

    #[tokio::test]
    async fn rejects_unknown_provider() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let providers = Arc::new(InMemoryExternalIdentityProviderRepository::new());
        let factory = factory_with_client(&client, providers);
        let use_case = UseCase::new(factory, Arc::new(StubSecretCipher));

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    oauth_client_id: client.id(),
                    provider: "facebook".to_string(),
                    client_id: "x".to_string(),
                    client_secret: "y".to_string(),
                    redirect_uri: "https://idp.example.com/cb".to_string(),
                    is_active: true,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }
}
