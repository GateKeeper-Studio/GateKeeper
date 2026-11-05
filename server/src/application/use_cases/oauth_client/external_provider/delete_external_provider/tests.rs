#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::external_provider::delete_external_provider::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                external_identity_provider::{ExternalIdentityProvider, ExternalProviderKind},
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
            },
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
        },
    };

    fn make_client(tenant_id: Uuid, external_login_only: bool) -> OAuthClient {
        let mut client = OAuthClient::new(
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
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        );
        client.set_external_login_only(external_login_only);
        client
    }

    fn make_provider(
        tenant_id: Uuid,
        oauth_client_id: Uuid,
        is_active: bool,
    ) -> ExternalIdentityProvider {
        ExternalIdentityProvider::new(
            oauth_client_id,
            tenant_id,
            ExternalProviderKind::Github,
            "gh".to_string(),
            "enc:secret".to_string(),
            "https://idp.example.com/cb/github".to_string(),
            is_active,
        )
    }

    // Builds a use case over a client + its providers, both pre-seeded.
    fn setup(
        client: &OAuthClient,
        providers: &[ExternalIdentityProvider],
    ) -> (UseCase, Arc<InMemoryExternalIdentityProviderRepository>) {
        let provider_repo = Arc::new(InMemoryExternalIdentityProviderRepository::new());
        {
            let mut stored = provider_repo.memory.data.lock().unwrap();
            for provider in providers {
                stored.insert(provider.id(), provider.clone());
            }
        }

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_oauth_client_repository(clients)
            .with_external_identity_provider_repository(Arc::clone(&provider_repo)),
        );

        (UseCase::new(factory), provider_repo)
    }

    #[tokio::test]
    async fn deletes_configured_provider() {
        let tenant_id = Uuid::now_v7();
        let oauth_client_id = Uuid::now_v7();

        let providers = Arc::new(InMemoryExternalIdentityProviderRepository::new());
        let provider = ExternalIdentityProvider::new(
            oauth_client_id,
            tenant_id,
            ExternalProviderKind::Google,
            "id".to_string(),
            "enc:secret".to_string(),
            "https://idp.example.com/cb/google".to_string(),
            true,
        );
        let provider_id = provider.id();
        providers
            .memory
            .data
            .lock()
            .unwrap()
            .insert(provider_id, provider);

        let factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_external_identity_provider_repository(Arc::clone(&providers)),
        );
        let use_case = UseCase::new(factory);

        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    oauth_client_id,
                    id: provider_id,
                },
            )
            .await
            .unwrap();

        assert!(providers.memory.data.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn not_found_for_unknown_provider() {
        let factory = Arc::new(InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        ));
        let use_case = UseCase::new(factory);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: Uuid::now_v7(),
                    oauth_client_id: Uuid::now_v7(),
                    id: Uuid::now_v7(),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    // The other half of the external-login-only invariant: update_oauth_client refuses
    // to turn the flag on without an active provider, and this refuses to take the last
    // one away afterwards. Together they mean the login page always offers a way in.
    #[tokio::test]
    async fn refuses_to_delete_the_last_active_provider_of_an_external_only_client() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, true);
        let provider = make_provider(tenant_id, client.id(), true);
        let provider_id = provider.id();
        let (use_case, providers) = setup(&client, &[provider]);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    oauth_client_id: client.id(),
                    id: provider_id,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidRequest(_)));
        assert!(
            providers
                .memory
                .data
                .lock()
                .unwrap()
                .contains_key(&provider_id)
        );
    }

    #[tokio::test]
    async fn allows_deleting_a_provider_when_another_active_one_remains() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, true);
        let first = make_provider(tenant_id, client.id(), true);
        let second = make_provider(tenant_id, client.id(), true);
        let first_id = first.id();
        let (use_case, providers) = setup(&client, &[first, second]);

        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    oauth_client_id: client.id(),
                    id: first_id,
                },
            )
            .await
            .unwrap();

        let stored = providers.memory.data.lock().unwrap();
        assert!(!stored.contains_key(&first_id));
        assert_eq!(stored.len(), 1);
    }

    // A client that still accepts passwords has another way in, so its last provider
    // is not load-bearing.
    #[tokio::test]
    async fn allows_deleting_the_last_provider_when_the_client_accepts_passwords() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, false);
        let provider = make_provider(tenant_id, client.id(), true);
        let provider_id = provider.id();
        let (use_case, providers) = setup(&client, &[provider]);

        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    oauth_client_id: client.id(),
                    id: provider_id,
                },
            )
            .await
            .unwrap();

        assert!(providers.memory.data.lock().unwrap().is_empty());
    }
}
