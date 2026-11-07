#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::shared::client_secret::hash_client_secret,
        application::use_cases::oauth_client::{
            create_oauth_client::{command::Command, use_case::UseCase},
            validation::LoginAppearanceInput,
        },
        domain::{
            entities::{oauth_client::LoginLayout, tenant::Tenant},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn make_tenant() -> Tenant {
        Tenant::new(
            "T".to_string(),
            None,
            "tenant_pepper".to_string(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
        )
    }

    fn setup(tenant: &Tenant) -> (Arc<InMemoryOAuthClientRepository>, UseCase) {
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        let tenants = Arc::new(InMemoryTenantRepository::new());
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let factory =
            InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenants)
                .with_oauth_client_repository(clients.clone());

        (clients, UseCase::new(Arc::new(factory), false))
    }

    fn base_command(tenant_id: Uuid, client_type: &str) -> Command {
        Command {
            tenant_id,
            name: "My App".to_string(),
            client_type: client_type.to_string(),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            post_logout_redirect_uris: vec![],
            allowed_scopes: vec!["openid".to_string(), "email".to_string()],
            allowed_grant_types: vec!["authorization_code".to_string()],
            require_pkce: true,
            require_consent: true,
            email_provider_id: None,
            tos_uri: None,
            policy_uri: None,
            initiate_login_uri: None,
            login_appearance: None,
            external_login_only: false,
            magic_link_enabled: false,
            backchannel_logout_uri: None,
            backchannel_logout_session_required: false,
            allowed_languages: vec!["en".to_string()],
            default_language: "en".to_string(),
            mfa_required: false,
            mfa_email_enabled: true,
            mfa_totp_enabled: true,
            mfa_webauthn_enabled: true,
        }
    }

    #[tokio::test]
    async fn creates_a_public_client_without_a_secret() {
        let tenant = make_tenant();
        let (clients, use_case) = setup(&tenant);

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                base_command(tenant.id(), "public"),
            )
            .await
            .unwrap();

        assert_eq!(response.client.client_type, "public");
        assert!(!response.client.client_id.is_empty());
        assert!(response.client_secret.is_none());
        assert_eq!(clients.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn stores_allowed_and_default_languages() {
        let tenant = make_tenant();
        let (clients, use_case) = setup(&tenant);

        let mut command = base_command(tenant.id(), "public");
        command.allowed_languages = vec!["en".to_string(), "es".to_string()];
        command.default_language = "es".to_string();

        use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap();

        let stored = clients.memory.data.lock().unwrap();
        let client = stored.values().next().unwrap();
        assert_eq!(
            client.allowed_languages(),
            &["en".to_string(), "es".to_string()]
        );
        assert_eq!(client.default_language(), "es");
    }

    #[tokio::test]
    async fn rejects_default_language_not_in_allowed_languages() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let mut command = base_command(tenant.id(), "public");
        command.allowed_languages = vec!["en".to_string()];
        command.default_language = "es".to_string();

        assert!(matches!(
            use_case
                .execute(&crate::application::shared::actor::Actor::System, command)
                .await,
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[tokio::test]
    async fn stores_normalized_legal_uris() {
        let tenant = make_tenant();
        let (clients, use_case) = setup(&tenant);

        let mut command = base_command(tenant.id(), "public");
        command.tos_uri = Some(" https://app.example.com/terms ".to_string());
        command.policy_uri = Some("https://app.example.com/privacy".to_string());

        let response = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap();

        assert_eq!(
            response.client.tos_uri.as_deref(),
            Some("https://app.example.com/terms")
        );
        assert_eq!(
            response.client.policy_uri.as_deref(),
            Some("https://app.example.com/privacy")
        );

        let stored = clients.memory.data.lock().unwrap();
        let client = stored.values().next().unwrap();
        assert_eq!(client.tos_uri(), Some("https://app.example.com/terms"));
        assert_eq!(client.policy_uri(), Some("https://app.example.com/privacy"));
    }

    #[tokio::test]
    async fn rejects_unsafe_legal_uri() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let mut command = base_command(tenant.id(), "public");
        command.policy_uri = Some("javascript:alert(1)".to_string());

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn stores_login_appearance() {
        let tenant = make_tenant();
        let (clients, use_case) = setup(&tenant);

        let mut command = base_command(tenant.id(), "public");
        command.login_appearance = Some(LoginAppearanceInput {
            layout: Some("card".into()),
            primary_color: Some("#123456".into()),
            ..Default::default()
        });

        let response = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap();
        let appearance = response
            .client
            .login_appearance
            .expect("appearance should be present in the view");
        assert_eq!(appearance.layout, "card");
        assert_eq!(appearance.primary_color.as_deref(), Some("#123456"));

        let stored = clients.memory.data.lock().unwrap();
        let client = stored.values().next().unwrap();
        assert_eq!(client.login_appearance().unwrap().layout, LoginLayout::Card);
    }

    #[tokio::test]
    async fn rejects_unsafe_login_appearance() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let mut command = base_command(tenant.id(), "public");
        command.login_appearance = Some(LoginAppearanceInput {
            layout: Some("centered".into()),
            logo_uri: Some("javascript:alert(1)".into()),
            ..Default::default()
        });

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn creates_a_confidential_client_and_returns_a_verifiable_secret_once() {
        let tenant = make_tenant();
        let (clients, use_case) = setup(&tenant);

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                base_command(tenant.id(), "confidential"),
            )
            .await
            .unwrap();

        let secret = response
            .client_secret
            .expect("confidential client must return a secret");
        // The stored hash verifies against the raw secret with the tenant pepper, and
        // the raw secret is never persisted.
        let stored = clients.memory.data.lock().unwrap();
        let client = stored.values().next().unwrap();
        let hash = client
            .client_secret_hash()
            .expect("secret hash should be stored");
        assert_eq!(
            hash,
            hash_client_secret(&secret, tenant.password_hash_secret())
        );
    }

    #[tokio::test]
    async fn rejects_invalid_redirect_uri() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let mut command = base_command(tenant.id(), "public");
        command.redirect_uris = vec!["http://evil.example.com/cb".to_string()]; // plain http, non-loopback

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn rejects_unknown_client_type() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                base_command(tenant.id(), "weird"),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn fails_when_tenant_missing() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                base_command(Uuid::now_v7(), "public"),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
