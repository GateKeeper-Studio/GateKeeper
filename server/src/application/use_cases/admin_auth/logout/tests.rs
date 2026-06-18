#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};

    use crate::{
        application::use_cases::admin_auth::logout::{command::Command, use_case::UseCase},
        domain::entities::admin_refresh_token::AdminRefreshToken,
        testing::in_memory_repositories::in_memory_admin_refresh_token_repository::InMemoryAdminRefreshTokenRepository,
        utils::oauth_crypto::{generate_opaque_token, sha256_hex},
    };

    #[tokio::test]
    async fn revokes_the_presented_tokens_family() {
        let repo = Arc::new(InMemoryAdminRefreshTokenRepository::new());
        let raw = generate_opaque_token();
        let stored = AdminRefreshToken::new(
            uuid::Uuid::now_v7(),
            sha256_hex(&raw),
            Utc::now() + Duration::days(30),
        );
        repo.tokens.lock().unwrap().push(stored);

        let use_case = UseCase::new(repo.clone());
        use_case
            .execute(Command { refresh_token: raw })
            .await
            .unwrap();

        let tokens = repo.tokens.lock().unwrap();
        assert!(tokens.iter().all(|t| t.revoked_at().is_some()));
    }

    #[tokio::test]
    async fn unknown_token_is_a_silent_no_op() {
        let repo = Arc::new(InMemoryAdminRefreshTokenRepository::new());
        let use_case = UseCase::new(repo.clone());

        // Never errors, even though the token doesn't exist.
        use_case
            .execute(Command {
                refresh_token: "nope".to_string(),
            })
            .await
            .unwrap();
        assert!(repo.tokens.lock().unwrap().is_empty());
    }
}
