#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::{
            update_oauth_client::{command::Command, use_case::UseCase},
            validation::LoginAppearanceInput,
        },
        domain::{
            entities::{
                external_identity_provider::{ExternalIdentityProvider, ExternalProviderKind},
                oauth_client::{ClientType, LoginLayout, MfaPolicy, OAuthClient},
                tenant::Tenant,
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

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::new(
            tenant_id,
            Uuid::now_v7(),
            None,
            ClientType::Public,
            "Old Name".to_string(),
            vec!["https://app.example.com/cb".to_string()],
            vec![],
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
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
            "gh-client".to_string(),
            "ciphertext".to_string(),
            "https://idp.example.com/v1/auth/oauth-provider/github/callback".to_string(),
            is_active,
        )
    }

    fn setup(
        tenant_id: Uuid,
        client: Option<&OAuthClient>,
    ) -> (Arc<InMemoryOAuthClientRepository>, UseCase) {
        let (clients, use_case, _) = setup_with_providers(tenant_id, client, &[]);
        (clients, use_case)
    }

    fn setup_with_providers(
        tenant_id: Uuid,
        client: Option<&OAuthClient>,
        providers: &[ExternalIdentityProvider],
    ) -> (
        Arc<InMemoryOAuthClientRepository>,
        UseCase,
        Arc<InMemoryExternalIdentityProviderRepository>,
    ) {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "T".into(),
            None,
            "s".into(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
            chrono::Utc::now(),
            None,
        );
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant_id, tenant);

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        if let Some(c) = client {
            clients
                .memory
                .data
                .lock()
                .unwrap()
                .insert(c.id(), c.clone());
        }

        let external_providers = Arc::new(InMemoryExternalIdentityProviderRepository::new());
        {
            let mut stored = external_providers.memory.data.lock().unwrap();
            for provider in providers {
                stored.insert(provider.id(), provider.clone());
            }
        }

        let factory =
            InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenants)
                .with_oauth_client_repository(clients.clone())
                .with_external_identity_provider_repository(external_providers.clone());
        (clients, UseCase::new(Arc::new(factory)), external_providers)
    }

    fn command(tenant_id: Uuid, id: Uuid) -> Command {
        Command {
            tenant_id,
            id,
            name: "New Name".to_string(),
            redirect_uris: vec!["https://app.example.com/cb2".to_string()],
            post_logout_redirect_uris: vec![],
            allowed_scopes: vec!["openid".to_string(), "profile".to_string()],
            allowed_grant_types: vec![
                "authorization_code".to_string(),
                "refresh_token".to_string(),
            ],
            require_pkce: true,
            require_consent: false,
            email_provider_id: None,
            tos_uri: None,
            policy_uri: None,
            initiate_login_uri: None,
            login_appearance: None,
            external_login_only: false,
            allowed_languages: vec!["en".to_string()],
            default_language: "en".to_string(),
            mfa_required: false,
            mfa_email_enabled: true,
            mfa_totp_enabled: true,
            mfa_webauthn_enabled: true,
            is_active: false,
        }
    }

    #[tokio::test]
    async fn updates_editable_details() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (clients, use_case) = setup(tenant_id, Some(&client));

        let view = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                command(tenant_id, client.id()),
            )
            .await
            .unwrap();
        assert_eq!(view.name, "New Name");
        assert_eq!(
            view.redirect_uris,
            vec!["https://app.example.com/cb2".to_string()]
        );
        assert!(!view.require_consent);
        assert!(!view.is_active);

        // Persisted, and the public client_id is unchanged (identity is immutable).
        let stored = clients.memory.data.lock().unwrap();
        let saved = stored.get(&client.id()).unwrap();
        assert_eq!(saved.client_id(), client.client_id());
        assert_eq!(saved.name(), "New Name");
    }

    #[tokio::test]
    async fn updates_allowed_and_default_languages() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(tenant_id, Some(&client));

        let mut cmd = command(tenant_id, client.id());
        cmd.allowed_languages = vec!["en".to_string(), "pt-br".to_string()];
        cmd.default_language = "pt-br".to_string();

        let view = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap();
        assert_eq!(
            view.allowed_languages,
            vec!["en".to_string(), "pt-br".to_string()]
        );
        assert_eq!(view.default_language, "pt-br");
    }

    #[tokio::test]
    async fn rejects_default_language_not_in_allowed_languages() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(tenant_id, Some(&client));

        let mut cmd = command(tenant_id, client.id());
        cmd.allowed_languages = vec!["en".to_string()];
        cmd.default_language = "es".to_string();

        assert!(matches!(
            use_case
                .execute(&crate::application::shared::actor::Actor::System, cmd)
                .await,
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[tokio::test]
    async fn updates_legal_uris() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (clients, use_case) = setup(tenant_id, Some(&client));

        let mut cmd = command(tenant_id, client.id());
        cmd.tos_uri = Some("https://app.example.com/terms".to_string());
        cmd.policy_uri = Some(" https://app.example.com/privacy ".to_string());

        let view = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap();

        assert_eq!(
            view.tos_uri.as_deref(),
            Some("https://app.example.com/terms")
        );
        assert_eq!(
            view.policy_uri.as_deref(),
            Some("https://app.example.com/privacy")
        );

        let stored = clients.memory.data.lock().unwrap();
        let saved = stored.get(&client.id()).unwrap();
        assert_eq!(saved.tos_uri(), Some("https://app.example.com/terms"));
        assert_eq!(saved.policy_uri(), Some("https://app.example.com/privacy"));
    }

    #[tokio::test]
    async fn rejects_unsafe_legal_uri() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(tenant_id, Some(&client));

        let mut cmd = command(tenant_id, client.id());
        cmd.tos_uri = Some("data:text/html,owned".to_string());

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn updates_login_appearance() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (clients, use_case) = setup(tenant_id, Some(&client));

        let mut cmd = command(tenant_id, client.id());
        cmd.login_appearance = Some(LoginAppearanceInput {
            layout: Some("minimal".into()),
            heading: Some(std::collections::HashMap::from([(
                "en".to_string(),
                "Hi there".to_string(),
            )])),
            ..Default::default()
        });

        let view = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap();
        let appearance = view
            .login_appearance
            .expect("appearance should be present in the view");
        assert_eq!(appearance.layout, "minimal");

        let stored = clients.memory.data.lock().unwrap();
        let saved = stored.get(&client.id()).unwrap();
        assert_eq!(
            saved.login_appearance().unwrap().layout,
            LoginLayout::Minimal
        );
        assert_eq!(
            saved
                .login_appearance()
                .unwrap()
                .heading
                .as_ref()
                .and_then(|m| m.get("en"))
                .map(String::as_str),
            Some("Hi there")
        );
    }

    #[tokio::test]
    async fn rejects_unsafe_login_appearance() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(tenant_id, Some(&client));

        let mut cmd = command(tenant_id, client.id());
        cmd.login_appearance = Some(LoginAppearanceInput {
            layout: Some("centered".into()),
            primary_color: Some("#fff;}body{display:none".into()),
            ..Default::default()
        });

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn not_found_for_unknown_client() {
        let tenant_id = Uuid::now_v7();
        let (_, use_case) = setup(tenant_id, None);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                command(tenant_id, Uuid::now_v7()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    // Turning external-login-only on with nothing to sign in through would leave the
    // hosted login page with no way in at all — every password path is refused and no
    // provider button renders.
    #[tokio::test]
    async fn rejects_external_login_only_without_an_active_provider() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case, _) = setup_with_providers(tenant_id, Some(&client), &[]);

        let mut cmd = command(tenant_id, client.id());
        cmd.external_login_only = true;

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    // An INACTIVE provider is not a way in either.
    #[tokio::test]
    async fn rejects_external_login_only_when_the_only_provider_is_inactive() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let provider = make_provider(tenant_id, client.id(), false);
        let (_, use_case, _) = setup_with_providers(tenant_id, Some(&client), &[provider]);

        let mut cmd = command(tenant_id, client.id());
        cmd.external_login_only = true;

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn enables_external_login_only_with_an_active_provider() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let provider = make_provider(tenant_id, client.id(), true);
        let (clients, use_case, _) = setup_with_providers(tenant_id, Some(&client), &[provider]);

        let mut cmd = command(tenant_id, client.id());
        cmd.external_login_only = true;

        let view = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap();
        assert!(view.external_login_only);

        let stored = clients.memory.data.lock().unwrap();
        let saved = stored.get(&client.id()).unwrap();
        assert!(saved.external_login_only());
        assert!(!saved.allows_password_login());
    }

    // Turning it back OFF needs no provider — it only ever adds a way in.
    #[tokio::test]
    async fn disables_external_login_only_without_any_provider() {
        let tenant_id = Uuid::now_v7();
        let mut client = make_client(tenant_id);
        client.set_external_login_only(true);
        let (clients, use_case, _) = setup_with_providers(tenant_id, Some(&client), &[]);

        let view = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                command(tenant_id, client.id()),
            )
            .await
            .unwrap();
        assert!(!view.external_login_only);

        let stored = clients.memory.data.lock().unwrap();
        assert!(stored.get(&client.id()).unwrap().allows_password_login());
    }

    #[tokio::test]
    async fn rejects_invalid_redirect_uri() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(tenant_id, Some(&client));

        let mut cmd = command(tenant_id, client.id());
        cmd.redirect_uris = vec!["not-a-uri".to_string()];

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }
}
