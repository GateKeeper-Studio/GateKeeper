#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::{
        application::use_cases::admin_account::change_password::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                admin_refresh_token::AdminRefreshToken, admin_web_session::AdminWebSession,
            },
            errors::DomainError,
        },
        testing::admin_fixture::{AdminFixture, PASSWORD, PEPPER},
        utils::password_hasher::compare_password,
    };

    const NEW: &str = "a-brand-new-password";

    fn use_case(f: &AdminFixture) -> UseCase {
        UseCase::new(
            f.admins.clone(),
            f.authenticator.clone(),
            f.refresh_tokens.clone(),
            f.web_sessions.clone(),
        )
    }

    fn command(f: &AdminFixture, current: &str, new: &str) -> Command {
        Command {
            admin_id: f.admin.id(),
            keep_session_id: None,
            current_password: current.to_string(),
            new_password: new.to_string(),
        }
    }

    #[tokio::test]
    async fn changes_the_password_and_ends_every_other_session() {
        let f = AdminFixture::new();
        let now = Utc::now();
        let current = AdminWebSession::new(f.admin.id(), "current".into(), now, Duration::hours(1));
        let other = AdminWebSession::new(f.admin.id(), "other".into(), now, Duration::hours(1));
        f.web_sessions
            .sessions
            .lock()
            .unwrap()
            .extend([current.clone(), other]);
        f.refresh_tokens
            .tokens
            .lock()
            .unwrap()
            .push(AdminRefreshToken::new(
                f.admin.id(),
                "refresh".into(),
                now + Duration::days(1),
            ));

        use_case(&f)
            .execute(Command {
                keep_session_id: Some(current.id()),
                ..command(&f, PASSWORD, NEW)
            })
            .await
            .unwrap();

        let stored = f.stored();
        assert!(compare_password(stored.password_hash(), NEW, PEPPER));
        assert!(!compare_password(stored.password_hash(), PASSWORD, PEPPER));

        let sessions = f.web_sessions.sessions.lock().unwrap().clone();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id(), current.id(), "the current session stays");
        assert!(
            f.refresh_tokens.tokens.lock().unwrap()[0]
                .revoked_at()
                .is_some()
        );
    }

    #[tokio::test]
    async fn rejects_a_wrong_current_password_and_counts_it() {
        let f = AdminFixture::new();

        let result = use_case(&f)
            .execute(command(&f, "not-the-password", NEW))
            .await;

        assert!(matches!(result, Err(DomainError::Validation(_))));
        let stored = f.stored();
        assert!(compare_password(stored.password_hash(), PASSWORD, PEPPER));
        assert_eq!(stored.failed_sign_in_count(), 1);
    }

    #[tokio::test]
    async fn the_new_password_must_meet_the_policy_and_differ() {
        let f = AdminFixture::new();
        for new in ["too-short", PASSWORD, "admin@example.com"] {
            let result = use_case(&f).execute(command(&f, PASSWORD, new)).await;
            assert!(matches!(result, Err(DomainError::Validation(_))), "{new}");
        }
        assert!(compare_password(
            f.stored().password_hash(),
            PASSWORD,
            PEPPER
        ));
    }
}
