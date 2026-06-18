#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::delete_oauth_client::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
            },
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

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::new(
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
            MfaPolicy::default(),
        )
    }

    fn setup(
        tenant_id: Uuid,
        client: Option<&OAuthClient>,
    ) -> (Arc<InMemoryOAuthClientRepository>, UseCase) {
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
            false,
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

        let factory =
            InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenants)
                .with_oauth_client_repository(clients.clone());
        (clients, UseCase::new(Arc::new(factory)))
    }

    #[tokio::test]
    async fn deletes_an_existing_client() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (clients, use_case) = setup(tenant_id, Some(&client));

        use_case
            .execute(Command {
                tenant_id,
                id: client.id(),
            })
            .await
            .unwrap();

        // The in-memory store flushes deletes on commit.
        assert!(
            !clients
                .memory
                .data
                .lock()
                .unwrap()
                .contains_key(&client.id())
        );
    }

    #[tokio::test]
    async fn not_found_for_unknown_client() {
        let tenant_id = Uuid::now_v7();
        let (_, use_case) = setup(tenant_id, None);

        let err = use_case
            .execute(Command {
                tenant_id,
                id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::OAuthClientNotFound(_)));
    }
}
