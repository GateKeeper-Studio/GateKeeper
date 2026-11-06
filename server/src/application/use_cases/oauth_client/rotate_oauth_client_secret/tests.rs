#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::shared::client_secret::hash_client_secret,
        application::use_cases::oauth_client::{
            create_oauth_client::{
                command::Command as CreateCommand, use_case::UseCase as CreateUseCase,
            },
            rotate_oauth_client_secret::{command::Command, use_case::UseCase},
        },
        domain::{entities::tenant::Tenant, errors::DomainError},
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

    fn setup(tenant: &Tenant) -> (Arc<InMemoryOAuthClientRepository>, CreateUseCase, UseCase) {
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        let tenants = Arc::new(InMemoryTenantRepository::new());
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenants)
                .with_oauth_client_repository(clients.clone()),
        );

        (
            clients,
            CreateUseCase::new(factory.clone()),
            UseCase::new(factory),
        )
    }

    fn create_command(tenant_id: Uuid, client_type: &str) -> CreateCommand {
        CreateCommand {
            tenant_id,
            name: "My App".to_string(),
            client_type: client_type.to_string(),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            post_logout_redirect_uris: vec![],
            allowed_scopes: vec!["openid".to_string()],
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
            allowed_languages: vec!["en".to_string()],
            default_language: "en".to_string(),
            mfa_required: false,
            mfa_email_enabled: true,
            mfa_totp_enabled: true,
            mfa_webauthn_enabled: true,
        }
    }

    #[tokio::test]
    async fn rotates_a_confidential_client_secret_and_returns_a_new_verifiable_one() {
        let tenant = make_tenant();
        let (clients, create, rotate) = setup(&tenant);

        let created = create
            .execute(
                &crate::application::shared::actor::Actor::System,
                create_command(tenant.id(), "confidential"),
            )
            .await
            .unwrap();
        let client_db_id = created.client.id;
        let old_client_id = created.client.client_id.clone();
        let old_secret = created
            .client_secret
            .expect("confidential client must return a secret");

        let rotated = rotate
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    id: client_db_id,
                },
            )
            .await
            .unwrap();

        // A brand new secret, and the public client_id is left untouched.
        assert_ne!(rotated.client_secret, old_secret);
        assert_eq!(rotated.client.client_id, old_client_id);

        // The stored hash now verifies the NEW secret and no longer the old one.
        let stored = clients.memory.data.lock().unwrap();
        let client = stored.values().next().unwrap();
        let hash = client
            .client_secret_hash()
            .expect("secret hash should be stored");
        assert_eq!(
            hash,
            hash_client_secret(&rotated.client_secret, tenant.password_hash_secret())
        );
        assert_ne!(
            hash,
            hash_client_secret(&old_secret, tenant.password_hash_secret())
        );
    }

    #[tokio::test]
    async fn rejects_rotation_for_public_clients() {
        let tenant = make_tenant();
        let (_, create, rotate) = setup(&tenant);

        let created = create
            .execute(
                &crate::application::shared::actor::Actor::System,
                create_command(tenant.id(), "public"),
            )
            .await
            .unwrap();

        let err = rotate
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    id: created.client.id,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn fails_when_client_missing() {
        let tenant = make_tenant();
        let (_, _create, rotate) = setup(&tenant);

        let err = rotate
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    id: Uuid::now_v7(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_tenant_missing() {
        let tenant = make_tenant();
        let (_, _create, rotate) = setup(&tenant);

        let err = rotate
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: Uuid::now_v7(),
                    id: Uuid::now_v7(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
