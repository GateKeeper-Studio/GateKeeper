#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};

    use crate::{
        application::use_cases::admin_auth::refresh::{command::Command, use_case::UseCase},
        domain::{
            entities::{
                admin_refresh_token::AdminRefreshToken,
                admin_user::AdminUser,
                audit_action::{ADMIN_TOKEN_REFRESHED, ADMIN_TOKEN_REUSE_DETECTED},
            },
            errors::DomainError,
        },
        testing::{
            fake_admin_token_issuer::FakeAdminTokenIssuer,
            fake_audit_logger::FakeAuditLogger,
            in_memory_repositories::{
                in_memory_admin_refresh_token_repository::InMemoryAdminRefreshTokenRepository,
                in_memory_admin_user_repository::InMemoryAdminUserRepository,
            },
        },
        utils::oauth_crypto::{generate_opaque_token, sha256_hex},
    };

    struct Ctx {
        use_case: UseCase,
        admin: AdminUser,
        refresh_repo: Arc<InMemoryAdminRefreshTokenRepository>,
        audit: FakeAuditLogger,
        raw_token: String,
    }

    fn setup(active_admin: bool) -> Ctx {
        let admin_repo = Arc::new(InMemoryAdminUserRepository::new());
        let refresh_repo = Arc::new(InMemoryAdminRefreshTokenRepository::new());
        let audit = FakeAuditLogger::new();

        let admin = AdminUser::reconstruct(
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
            "admin@example.com".to_string(),
            "Ada Admin".to_string(),
            "hash".to_string(),
            "argon2id".to_string(),
            active_admin,
            Utc::now(),
            None,
        );
        admin_repo.admins.lock().unwrap().push(admin.clone());

        // Seed a live refresh token for this admin.
        let raw_token = generate_opaque_token();
        let stored = AdminRefreshToken::new(
            admin.id(),
            sha256_hex(&raw_token),
            Utc::now() + Duration::days(30),
        );
        refresh_repo.tokens.lock().unwrap().push(stored);

        let use_case = UseCase::new(
            admin_repo,
            refresh_repo.clone(),
            Arc::new(FakeAdminTokenIssuer::new()),
            Arc::new(audit.clone()),
        );

        Ctx {
            use_case,
            admin,
            refresh_repo,
            audit,
            raw_token,
        }
    }

    fn command(token: &str) -> Command {
        Command {
            refresh_token: token.to_string(),
        }
    }

    #[tokio::test]
    async fn rotates_and_issues_a_fresh_access_token() {
        let ctx = setup(true);

        let response = ctx.use_case.execute(command(&ctx.raw_token)).await.unwrap();

        assert_eq!(response.token_type, "Bearer");
        assert!(response.expires_in > 0);
        assert!(response.access_token.contains(&ctx.admin.id().to_string()));
        // A NEW refresh token is returned (rotation), different from the presented one.
        assert_ne!(response.refresh_token, ctx.raw_token);
        assert_eq!(ctx.audit.count_action(ADMIN_TOKEN_REFRESHED), 1);

        // The presented token is now rotated (inactive); its successor is live.
        let tokens = ctx.refresh_repo.tokens.lock().unwrap();
        assert_eq!(tokens.len(), 2);
        let presented = tokens
            .iter()
            .find(|t| t.token_hash() == sha256_hex(&ctx.raw_token))
            .unwrap();
        assert!(presented.rotated_at().is_some());
    }

    #[tokio::test]
    async fn the_new_token_can_itself_be_refreshed() {
        let ctx = setup(true);
        let first = ctx.use_case.execute(command(&ctx.raw_token)).await.unwrap();

        let second = ctx
            .use_case
            .execute(command(&first.refresh_token))
            .await
            .unwrap();

        assert_ne!(second.refresh_token, first.refresh_token);
    }

    #[tokio::test]
    async fn rejects_an_unknown_token() {
        let ctx = setup(true);
        let err = ctx
            .use_case
            .execute(command("not-a-real-token"))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
    }

    #[tokio::test]
    async fn detects_reuse_and_revokes_the_whole_family() {
        let ctx = setup(true);
        // First refresh rotates the original token away.
        let first = ctx.use_case.execute(command(&ctx.raw_token)).await.unwrap();

        // Presenting the ALREADY-ROTATED original token again is the theft signal.
        let err = ctx
            .use_case
            .execute(command(&ctx.raw_token))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));
        assert_eq!(ctx.audit.count_action(ADMIN_TOKEN_REUSE_DETECTED), 1);

        // The whole family is dead now: even the (previously valid) successor fails.
        let err2 = ctx
            .use_case
            .execute(command(&first.refresh_token))
            .await
            .unwrap_err();
        assert!(matches!(err2, DomainError::AuthenticationError(_)));

        let tokens = ctx.refresh_repo.tokens.lock().unwrap();
        assert!(tokens.iter().all(|t| t.revoked_at().is_some()));
    }

    #[tokio::test]
    async fn rejects_when_the_admin_is_no_longer_active() {
        let ctx = setup(false);

        let err = ctx
            .use_case
            .execute(command(&ctx.raw_token))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AuthenticationError(_)));

        // The family is revoked so the deactivated admin can't keep refreshing.
        let tokens = ctx.refresh_repo.tokens.lock().unwrap();
        assert!(tokens.iter().all(|t| t.revoked_at().is_some()));
    }
}
