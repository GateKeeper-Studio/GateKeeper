#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::external_provider::update_external_provider::{
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
            stub_secret_cipher::StubSecretCipher,
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

    fn make_provider(tenant_id: Uuid, oauth_client_id: Uuid) -> ExternalIdentityProvider {
        ExternalIdentityProvider::new(
            oauth_client_id,
            tenant_id,
            ExternalProviderKind::Github,
            "gh".to_string(),
            "enc:secret".to_string(),
            "https://idp.example.com/cb/github".to_string(),
            true,
        )
    }

    fn deactivate(tenant_id: Uuid, oauth_client_id: Uuid, id: Uuid) -> Command {
        Command {
            tenant_id,
            oauth_client_id,
            id,
            client_id: "gh".to_string(),
            client_secret: None,
            redirect_uri: "https://idp.example.com/cb/github".to_string(),
            is_active: false,
        }
    }

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

        (
            UseCase::new(factory, Arc::new(StubSecretCipher)),
            provider_repo,
        )
    }

    #[tokio::test]
    async fn keeps_secret_when_not_supplied_and_updates_fields() {
        let tenant_id = Uuid::now_v7();
        let oauth_client_id = Uuid::now_v7();

        let providers = Arc::new(InMemoryExternalIdentityProviderRepository::new());
        let provider = ExternalIdentityProvider::new(
            oauth_client_id,
            tenant_id,
            ExternalProviderKind::Github,
            "old-id".to_string(),
            "enc:old-secret".to_string(),
            "https://idp.example.com/cb/github".to_string(),
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
        let use_case = UseCase::new(factory, Arc::new(StubSecretCipher));

        let view = use_case
            .execute(Command {
                tenant_id,
                oauth_client_id,
                id: provider_id,
                client_id: "new-id".to_string(),
                client_secret: None,
                redirect_uri: "https://idp.example.com/cb/github".to_string(),
                is_active: false,
            })
            .await
            .unwrap();

        assert_eq!(view.provider_client_id, "new-id");
        assert!(!view.is_active);

        let stored = providers.memory.data.lock().unwrap();
        let saved = stored.get(&provider_id).unwrap();
        // Secret preserved (not rotated) since none was supplied.
        assert_eq!(saved.provider_client_secret_encrypted(), "enc:old-secret");
    }

    // Deactivating is as good as deleting from the login page's point of view, so it
    // hits the same external-login-only invariant as delete_external_provider.
    #[tokio::test]
    async fn refuses_to_deactivate_the_last_active_provider_of_an_external_only_client() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, true);
        let provider = make_provider(tenant_id, client.id());
        let provider_id = provider.id();
        let (use_case, providers) = setup(&client, &[provider]);

        let err = use_case
            .execute(deactivate(tenant_id, client.id(), provider_id))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidRequest(_)));
        let stored = providers.memory.data.lock().unwrap();
        assert!(stored.get(&provider_id).unwrap().is_active());
    }

    #[tokio::test]
    async fn allows_deactivating_when_another_active_provider_remains() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, true);
        let first = make_provider(tenant_id, client.id());
        let second = make_provider(tenant_id, client.id());
        let first_id = first.id();
        let (use_case, providers) = setup(&client, &[first, second]);

        use_case
            .execute(deactivate(tenant_id, client.id(), first_id))
            .await
            .unwrap();

        let stored = providers.memory.data.lock().unwrap();
        assert!(!stored.get(&first_id).unwrap().is_active());
    }

    // Editing an ALREADY-inactive provider must not trip the guard — it was never one
    // of the client's ways in.
    #[tokio::test]
    async fn allows_editing_an_already_inactive_provider() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, true);
        let mut provider = make_provider(tenant_id, client.id());
        provider.update_details(
            "gh".to_string(),
            "enc:secret".to_string(),
            "https://idp.example.com/cb/github".to_string(),
            false,
        );
        let provider_id = provider.id();
        let (use_case, _) = setup(&client, &[provider]);

        let view = use_case
            .execute(deactivate(tenant_id, client.id(), provider_id))
            .await
            .unwrap();
        assert!(!view.is_active);
    }
}
