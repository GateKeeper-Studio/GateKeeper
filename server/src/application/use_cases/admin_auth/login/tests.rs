#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::admin_auth::login::{command::Command, use_case::UseCase},
        domain::{entities::admin_user::AdminUser, errors::DomainError},
        testing::{
            fake_admin_token_issuer::FakeAdminTokenIssuer,
            in_memory_repositories::{
                in_memory_admin_refresh_token_repository::InMemoryAdminRefreshTokenRepository,
                in_memory_admin_user_repository::InMemoryAdminUserRepository,
            },
        },
        utils::password_hasher::hash_password,
    };

    const PEPPER: &str = "test-server-wide-pepper";
    const EMAIL: &str = "admin@example.com";
    const PASSWORD: &str = "Sup3rSecret!";

    struct Ctx {
        use_case: UseCase,
        repo: Arc<InMemoryAdminUserRepository>,
        refresh_repo: Arc<InMemoryAdminRefreshTokenRepository>,
    }

    fn setup(is_active: bool) -> Ctx {
        let repo = Arc::new(InMemoryAdminUserRepository::new());
        let refresh_repo = Arc::new(InMemoryAdminRefreshTokenRepository::new());
        let hash = hash_password(PASSWORD, PEPPER).unwrap();
        let mut admin = AdminUser::new(
            uuid::Uuid::now_v7(),
            EMAIL.to_string(),
            "Ada Admin".to_string(),
            hash,
            "argon2id".to_string(),
        );
        if !is_active {
            admin = AdminUser::reconstruct(
                admin.id(),
                admin.organization_id(),
                admin.email().to_string(),
                admin.name().to_string(),
                admin.password_hash().to_string(),
                admin.password_algorithm().to_string(),
                false,
                admin.created_at(),
                admin.updated_at(),
            );
        }
        repo.admins.lock().unwrap().push(admin);

        let use_case = UseCase::new(
            repo.clone(),
            refresh_repo.clone(),
            Arc::new(FakeAdminTokenIssuer::new()),
            PEPPER.to_string(),
            crate::testing::password_hasher::test_password_hasher(),
        );
        Ctx {
            use_case,
            repo,
            refresh_repo,
        }
    }

    fn command(email: &str, password: &str) -> Command {
        Command {
            email: email.to_string(),
            password: password.to_string(),
        }
    }

    #[tokio::test]
    async fn issues_a_token_on_valid_credentials() {
        let ctx = setup(true);

        let response = ctx
            .use_case
            .execute(command(EMAIL, PASSWORD))
            .await
            .unwrap();

        assert_eq!(response.token_type, "Bearer");
        assert!(response.expires_in > 0);
        let admin_id = ctx.repo.admins.lock().unwrap()[0].id();
        assert!(response.access_token.contains(&admin_id.to_string()));

        // A refresh token is returned (raw) and persisted (hashed, so it never equals raw).
        assert!(!response.refresh_token.is_empty());
        let stored = ctx.refresh_repo.tokens.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].admin_id(), admin_id);
        assert_ne!(stored[0].token_hash(), response.refresh_token);
    }

    #[tokio::test]
    async fn rejects_wrong_password() {
        let ctx = setup(true);
        let err = ctx
            .use_case
            .execute(command(EMAIL, "wrong-password"))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn rejects_unknown_email() {
        let ctx = setup(true);
        let err = ctx
            .use_case
            .execute(command("nobody@example.com", PASSWORD))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn rejects_inactive_admin() {
        let ctx = setup(false);
        let err = ctx
            .use_case
            .execute(command(EMAIL, PASSWORD))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }
}
