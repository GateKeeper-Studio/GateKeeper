#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};

    use crate::{
        application::{
            ports::admin_token_issuer::AdminTokenIssuer,
            use_cases::admin_web_session::resolve::{query::Query, use_case::UseCase},
        },
        domain::{
            entities::{admin_user::AdminUser, admin_web_session::AdminWebSession},
            errors::DomainError,
        },
        testing::{
            fake_admin_token_issuer::FakeAdminTokenIssuer,
            in_memory_repositories::{
                in_memory_admin_user_repository::InMemoryAdminUserRepository,
                in_memory_admin_web_session_repository::InMemoryAdminWebSessionRepository,
            },
        },
        utils::oauth_crypto::sha256_hex,
    };

    const TOKEN: &str = "raw-cookie-value";

    struct Ctx {
        use_case: UseCase,
        admins: Arc<InMemoryAdminUserRepository>,
        sessions: Arc<InMemoryAdminWebSessionRepository>,
        admin: AdminUser,
    }

    fn setup() -> Ctx {
        let admins = Arc::new(InMemoryAdminUserRepository::new());
        let sessions = Arc::new(InMemoryAdminWebSessionRepository::new());
        let admin = AdminUser::new(
            uuid::Uuid::now_v7(),
            "ada@example.com".to_string(),
            "Ada".to_string(),
            "hash".to_string(),
            "argon2id".to_string(),
        );
        admins.admins.lock().unwrap().push(admin.clone());
        let use_case = UseCase::new(
            admins.clone(),
            sessions.clone(),
            Arc::new(FakeAdminTokenIssuer::new()),
            crate::testing::admin_fixture::authenticator_for(admins.clone()),
        );
        Ctx {
            use_case,
            admins,
            sessions,
            admin,
        }
    }

    // A stored session whose clocks are given relative to now.
    fn store(ctx: &Ctx, last_seen_ago: Duration, expires_in: Duration) {
        let now = Utc::now();
        ctx.sessions
            .sessions
            .lock()
            .unwrap()
            .push(AdminWebSession::reconstruct(
                uuid::Uuid::now_v7(),
                ctx.admin.id(),
                sha256_hex(TOKEN),
                now - Duration::hours(1),
                now - last_seen_ago,
                now + expires_in,
            ));
    }

    fn query(token: &str) -> Query {
        Query {
            session_token: token.to_string(),
        }
    }

    fn assert_expired(result: Result<impl std::fmt::Debug, DomainError>) {
        let err = result.unwrap_err();
        assert!(
            matches!(&err, DomainError::AuthenticationError(m) if m == "session expired"),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn a_live_session_yields_a_fresh_admin_token_for_its_admin() {
        let ctx = setup();
        store(&ctx, Duration::minutes(5), Duration::hours(6));

        let response = ctx.use_case.execute(query(TOKEN)).await.unwrap();

        let claims = FakeAdminTokenIssuer::new()
            .verify(&response.access_token)
            .unwrap();
        assert_eq!(claims.admin_id, ctx.admin.id());
        assert_eq!(response.admin.email, "ada@example.com");
        // Touched: it was last seen five minutes ago, beyond the one-minute granularity.
        let last_seen = ctx.sessions.sessions.lock().unwrap()[0].last_seen_at();
        assert!(Utc::now() - last_seen < Duration::seconds(5));
    }

    #[tokio::test]
    async fn an_unknown_cookie_is_refused() {
        let ctx = setup();
        store(&ctx, Duration::minutes(1), Duration::hours(6));
        assert_expired(ctx.use_case.execute(query("forged")).await);
    }

    #[tokio::test]
    async fn an_idle_or_expired_session_is_refused_and_deleted() {
        for (last_seen_ago, expires_in) in [
            (Duration::hours(3), Duration::hours(6)),
            (Duration::minutes(1), Duration::seconds(-1)),
        ] {
            let ctx = setup();
            store(&ctx, last_seen_ago, expires_in);
            assert_expired(ctx.use_case.execute(query(TOKEN)).await);
            assert!(ctx.sessions.sessions.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn a_deactivated_admin_loses_the_session_on_the_next_request() {
        let ctx = setup();
        store(&ctx, Duration::minutes(1), Duration::hours(6));
        {
            let mut admins = ctx.admins.admins.lock().unwrap();
            admins[0].deactivate();
        }

        assert_expired(ctx.use_case.execute(query(TOKEN)).await);
        assert!(ctx.sessions.sessions.lock().unwrap().is_empty());
    }
}
