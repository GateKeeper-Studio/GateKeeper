#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::{
        application::use_cases::admin_account::deactivate_account::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                admin_refresh_token::AdminRefreshToken, admin_web_session::AdminWebSession,
            },
            errors::DomainError,
        },
        testing::admin_fixture::{AdminFixture, PASSWORD},
    };

    fn use_case(f: &AdminFixture) -> UseCase {
        UseCase::new(
            f.admins.clone(),
            f.authenticator.clone(),
            f.refresh_tokens.clone(),
            f.web_sessions.clone(),
        )
    }

    fn command(f: &AdminFixture, password: Option<&str>) -> Command {
        Command {
            admin_id: f.admin.id(),
            current_password: password.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn deactivates_and_ends_every_session() {
        let f = AdminFixture::new();
        f.add_colleague("colleague@example.com");
        let now = Utc::now();
        f.refresh_tokens
            .tokens
            .lock()
            .unwrap()
            .push(AdminRefreshToken::new(
                f.admin.id(),
                "refresh".into(),
                now + Duration::days(1),
            ));
        f.web_sessions
            .sessions
            .lock()
            .unwrap()
            .push(AdminWebSession::new(
                f.admin.id(),
                "session".into(),
                now,
                Duration::hours(1),
            ));

        use_case(&f)
            .execute(command(&f, Some(PASSWORD)))
            .await
            .unwrap();

        assert!(!f.stored().is_active());
        assert!(
            f.refresh_tokens.tokens.lock().unwrap()[0]
                .revoked_at()
                .is_some()
        );
        assert!(f.web_sessions.sessions.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn requires_the_current_password() {
        let f = AdminFixture::new();
        f.add_colleague("colleague@example.com");
        let result = use_case(&f).execute(command(&f, None)).await;
        assert!(matches!(result, Err(DomainError::StepUpRequired(_))));
        assert!(f.stored().is_active());
    }

    #[tokio::test]
    async fn refuses_to_deactivate_the_last_active_admin() {
        let f = AdminFixture::new();
        let result = use_case(&f).execute(command(&f, Some(PASSWORD))).await;
        assert!(matches!(result, Err(DomainError::AccessDenied(_))));
        assert!(f.stored().is_active());
    }

    #[tokio::test]
    async fn an_inactive_admin_is_an_idempotent_success() {
        let f = AdminFixture::new();
        let mut inactive = f.stored();
        inactive.deactivate();
        *f.admins.admins.lock().unwrap().first_mut().unwrap() = inactive;
        use_case(&f).execute(command(&f, None)).await.unwrap();
    }
}
