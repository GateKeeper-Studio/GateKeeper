#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth::userinfo::{query::Query, use_case::UseCase},
        domain::{
            entities::signing_key::{SigningAlgorithm, SigningKey},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_signing_key_repository::InMemorySigningKeyRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            oauth_fakes::FakeTokenVerifier,
            user_factory::generate_user,
        },
    };

    const TOKEN: &str = "valid-access-token";

    fn setup(scopes: Vec<String>) -> (UseCase, Uuid, Uuid) {
        let tenant_id = Uuid::now_v7();
        let user = generate_user(tenant_id);
        let user_id = user.id();

        let users = Arc::new(InMemoryUserRepository::new());
        users.memory.data.lock().unwrap().insert(user.id(), user);

        let keys = Arc::new(InMemorySigningKeyRepository::new());
        let key = SigningKey::new(
            tenant_id,
            "kid-1".to_string(),
            SigningAlgorithm::RS256,
            "{}".to_string(),
            "secret".to_string(),
        );
        keys.memory.data.lock().unwrap().insert(key.id(), key);

        let verifier =
            FakeTokenVerifier::new().with_valid_token(TOKEN, &user_id.to_string(), scopes);

        let use_case = UseCase::new(users, keys, Arc::new(verifier));
        (use_case, tenant_id, user_id)
    }

    #[tokio::test]
    async fn returns_only_scope_allowed_claims() {
        let (use_case, tenant_id, user_id) = setup(vec!["openid".to_string(), "email".to_string()]);

        let response = use_case
            .execute(Query {
                tenant_id,
                access_token: TOKEN.to_string(),
            })
            .await
            .unwrap();

        assert_eq!(response.sub, user_id.to_string());
        // email scope granted → email present
        assert!(response.email.is_some());
        assert_eq!(response.email_verified, Some(false));
        // profile scope NOT granted → name fields absent
        assert!(response.name.is_none());
        assert!(response.given_name.is_none());
    }

    #[tokio::test]
    async fn returns_profile_claims_when_profile_scope_present() {
        let (use_case, tenant_id, _) = setup(vec!["openid".to_string(), "profile".to_string()]);

        let response = use_case
            .execute(Query {
                tenant_id,
                access_token: TOKEN.to_string(),
            })
            .await
            .unwrap();

        assert!(response.given_name.is_some());
        assert!(response.family_name.is_some());
        // email scope NOT granted
        assert!(response.email.is_none());
    }

    #[tokio::test]
    async fn rejects_forged_or_unknown_token() {
        let (use_case, tenant_id, _) = setup(vec!["openid".to_string()]);

        let err = use_case
            .execute(Query {
                tenant_id,
                access_token: "forged".to_string(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::AccessDenied(_)));
    }
}
