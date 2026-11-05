#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::list_oauth_clients::{
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
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
    }

    #[tokio::test]
    async fn lists_only_this_tenants_clients_with_paging() {
        let tenant_id = Uuid::now_v7();
        let other = Uuid::now_v7();

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
        {
            let mut data = clients.memory.data.lock().unwrap();
            for c in [
                make_client(tenant_id, "a"),
                make_client(tenant_id, "b"),
                make_client(other, "x"),
            ] {
                data.insert(c.id(), c);
            }
        }

        let use_case = UseCase::new(clients, tenants);
        let page = use_case
            .execute(Query {
                tenant_id,
                page: Some(1),
                page_size: Some(10),
            })
            .await
            .unwrap();

        assert_eq!(page.total, 2);
        assert_eq!(page.items.len(), 2);
        assert!(
            page.items
                .iter()
                .all(|c| Uuid::parse_str(&c.client_id).is_ok())
        );
    }

    #[tokio::test]
    async fn fails_when_tenant_missing() {
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let use_case = UseCase::new(clients, tenants);

        let err = use_case
            .execute(Query {
                tenant_id: Uuid::now_v7(),
                page: None,
                page_size: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
