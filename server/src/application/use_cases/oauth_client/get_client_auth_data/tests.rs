#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::get_client_auth_data::{
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

    fn make_tenant(id: Uuid, is_active: bool, can_register: bool, can_forgot: bool) -> Tenant {
        Tenant::reconstruct(
            id,
            "T".to_string(),
            None,
            "s".to_string(),
            is_active,
            can_register,
            can_forgot,
            uuid::Uuid::now_v7(),
            false,
            chrono::Utc::now(),
            None,
        )
    }

    fn client_id() -> Uuid {
        Uuid::parse_str("01890000-0000-7000-8000-000000000001").unwrap()
    }

    fn make_client(tenant_id: Uuid, client_id: Uuid, is_active: bool) -> OAuthClient {
        OAuthClient::reconstruct(
            Uuid::now_v7(),
            tenant_id,
            client_id,
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
            Some("https://app.example.com/terms".to_string()),
            Some("https://app.example.com/privacy".to_string()),
            MfaPolicy::default(),
            is_active,
            chrono::Utc::now(),
            None,
        )
    }

    fn setup(tenant: Option<Tenant>, client: Option<&OAuthClient>) -> UseCase {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        if let Some(t) = tenant {
            tenants.memory.data.lock().unwrap().insert(t.id(), t);
        }

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
    async fn returns_public_auth_data_with_tenant_flags() {
        let tenant_id = Uuid::now_v7();
        let tenant = make_tenant(tenant_id, true, true, false);
        let client = make_client(tenant_id, client_id(), true);
        let use_case = setup(Some(tenant), Some(&client));

        let view = use_case
            .execute(Query {
                client_id: client_id().to_string(),
            })
            .await
            .unwrap();

        assert_eq!(view.id, client_id().to_string());
        assert_eq!(view.name, "App");
        assert!(view.can_self_sign_up);
        assert!(!view.can_self_forgot_pass);
        assert_eq!(
            view.tos_uri.as_deref(),
            Some("https://app.example.com/terms")
        );
        assert_eq!(
            view.policy_uri.as_deref(),
            Some("https://app.example.com/privacy")
        );
        assert!(view.oauth_providers.is_empty());
    }

    #[tokio::test]
    async fn not_found_for_unknown_client() {
        let tenant_id = Uuid::now_v7();
        let use_case = setup(Some(make_tenant(tenant_id, true, true, true)), None);

        let err = use_case
            .execute(Query {
                client_id: "nope".to_string(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::OAuthClientNotFound(_)));
    }

    #[tokio::test]
    async fn not_found_when_client_inactive() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, client_id(), false);
        let use_case = setup(
            Some(make_tenant(tenant_id, true, true, true)),
            Some(&client),
        );

        let err = use_case
            .execute(Query {
                client_id: client_id().to_string(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::OAuthClientNotFound(_)));
    }

    #[tokio::test]
    async fn not_found_when_tenant_inactive() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, client_id(), true);
        let use_case = setup(
            Some(make_tenant(tenant_id, false, true, true)),
            Some(&client),
        );

        let err = use_case
            .execute(Query {
                client_id: client_id().to_string(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::TenantNotFound(_)));
    }
}
