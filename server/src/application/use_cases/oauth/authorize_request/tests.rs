#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth::authorize_request::{query::Query, use_case::UseCase},
        domain::{
            entities::oauth_client::{ClientType, MfaPolicy, OAuthClient},
            errors::DomainError,
        },
        testing::in_memory_repositories::in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
    };

    fn client_id() -> Uuid {
        Uuid::parse_str("01890000-0000-7000-8000-000000000abc").unwrap()
    }

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::new(
            tenant_id,
            client_id(),
            None,
            ClientType::Public,
            "Test App".to_string(),
            vec!["https://app.example.com/callback".to_string()],
            vec![],
            vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            vec!["authorization_code".to_string()],
            true, // require_pkce
            true, // require_consent
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
    }

    fn setup(client: &OAuthClient) -> (Arc<InMemoryOAuthClientRepository>, UseCase) {
        let repo = Arc::new(InMemoryOAuthClientRepository::new());
        repo.memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());
        let use_case = UseCase::new(repo.clone());
        (repo, use_case)
    }

    fn make_query(tenant_id: Uuid) -> Query {
        Query {
            tenant_id,
            client_id: client_id().to_string(),
            redirect_uri: "https://app.example.com/callback".to_string(),
            response_type: "code".to_string(),
            scope: "openid email".to_string(),
            state: "xyz".to_string(),
            code_challenge: "abc123".to_string(),
            code_challenge_method: "S256".to_string(),
            nonce: Some("n-1".to_string()),
        }
    }

    #[tokio::test]
    async fn validates_a_well_formed_request() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(&client);

        let response = use_case.execute(make_query(tenant_id)).await.unwrap();

        assert_eq!(response.client_id, client_id().to_string());
        assert_eq!(
            response.scopes,
            vec!["openid".to_string(), "email".to_string()]
        );
        assert!(response.requires_consent);
    }

    #[tokio::test]
    async fn rejects_unknown_client() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(&client);

        let mut query = make_query(tenant_id);
        query.client_id = "nope".to_string();

        let err = use_case.execute(query).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidClient(_)));
    }

    #[tokio::test]
    async fn rejects_non_exact_redirect_uri() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(&client);

        let mut query = make_query(tenant_id);
        // Same prefix, extra path — exact match must reject this (open-redirect guard).
        query.redirect_uri = "https://app.example.com/callback/evil".to_string();

        let err = use_case.execute(query).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn rejects_plain_pkce_method() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(&client);

        let mut query = make_query(tenant_id);
        query.code_challenge_method = "plain".to_string();

        let err = use_case.execute(query).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn drops_out_of_scope_scopes_and_rejects_when_none_left() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(&client);

        let mut query = make_query(tenant_id);
        query.scope = "admin reports".to_string(); // none are allowed

        let err = use_case.execute(query).await.unwrap_err();
        assert!(matches!(err, DomainError::InvalidScope(_)));
    }

    #[tokio::test]
    async fn intersects_requested_scopes_with_allow_list() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);
        let (_, use_case) = setup(&client);

        let mut query = make_query(tenant_id);
        query.scope = "openid admin profile".to_string(); // admin is dropped

        let response = use_case.execute(query).await.unwrap();
        assert_eq!(
            response.scopes,
            vec!["openid".to_string(), "profile".to_string()]
        );
    }
}
