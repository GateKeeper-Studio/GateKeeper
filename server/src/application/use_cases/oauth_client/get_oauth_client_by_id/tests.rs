#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::get_oauth_client_by_id::{
            query::Query, use_case::UseCase,
        },
        domain::{
            entities::{
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::in_memory_repositories::{
            in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
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
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
    }

    fn setup(tenant_id: Uuid, client: Option<&OAuthClient>) -> UseCase {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "T".to_string(),
            None,
            "s".to_string(),
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
        UseCase::new(clients, tenants)
    }

    #[tokio::test]
    async fn returns_the_client_when_found() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let use_case = setup(tenant_id, Some(&client));

        let view = use_case
            .execute(Query {
                tenant_id,
                id: client.id(),
            })
            .await
            .unwrap();
        assert_eq!(view.id, client.id());
        assert_eq!(view.client_id, client.client_id().to_string());
    }

    #[tokio::test]
    async fn not_found_for_unknown_id() {
        let tenant_id = Uuid::now_v7();
        let use_case = setup(tenant_id, None);

        let err = use_case
            .execute(Query {
                tenant_id,
                id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn not_found_across_tenant_boundary() {
        let tenant_id = Uuid::now_v7();
        let other_tenant = Uuid::now_v7();
        let client = make_client(other_tenant); // belongs to a different tenant
        let use_case = setup(tenant_id, Some(&client));

        let err = use_case
            .execute(Query {
                tenant_id,
                id: client.id(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
