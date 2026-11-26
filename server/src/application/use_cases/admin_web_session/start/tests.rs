#[cfg(test)]
mod tests {
    use crate::{
        application::use_cases::admin_web_session::start::{
            command::Command,
            response::{Outcome, Response},
            use_case::UseCase,
        },
        domain::errors::DomainError,
        testing::admin_fixture::{AdminFixture, EMAIL, PASSWORD},
        utils::oauth_crypto::sha256_hex,
    };

    fn use_case(f: &AdminFixture) -> UseCase {
        UseCase::new(f.authenticator.clone(), f.web_sessions.clone())
    }

    fn command(email: &str, password: &str) -> Command {
        Command {
            email: email.to_string(),
            password: password.to_string(),
            mfa_code: None,
        }
    }

    fn started(outcome: Outcome) -> Response {
        match outcome {
            Outcome::Started(response) => response,
            other => panic!("expected a session, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signing_in_stores_only_the_hash_of_the_session_id() {
        let f = AdminFixture::new();

        let response = started(
            use_case(&f)
                .execute(command(EMAIL, PASSWORD))
                .await
                .unwrap(),
        );

        let stored = f.web_sessions.sessions.lock().unwrap().clone();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].admin_id(), f.admin.id());
        assert_eq!(stored[0].token_hash(), sha256_hex(&response.session_token));
        assert_ne!(stored[0].token_hash(), response.session_token);
        assert_eq!(response.admin.name, "Ada Admin");
        assert_eq!(response.admin.initials, "AA");
        assert_eq!(
            response.admin.organization_id,
            f.admin.organization_id().to_string()
        );
    }

    #[tokio::test]
    async fn an_empty_name_falls_back_to_the_email() {
        let f = AdminFixture::new();
        let mut unnamed = f.stored();
        unnamed.rename("  ".to_string());
        *f.admins.admins.lock().unwrap().first_mut().unwrap() = unnamed;

        let response = started(
            use_case(&f)
                .execute(command(EMAIL, PASSWORD))
                .await
                .unwrap(),
        );
        assert_eq!(response.admin.name, "Admin");
    }

    #[tokio::test]
    async fn wrong_password_and_unknown_email_fail_the_same_way_and_store_nothing() {
        let f = AdminFixture::new();

        for (email, password) in [(EMAIL, "wrong"), ("nobody@example.com", PASSWORD)] {
            let err = use_case(&f)
                .execute(command(email, password))
                .await
                .unwrap_err();
            assert!(
                matches!(&err, DomainError::AuthenticationError(m) if m == "invalid credentials"),
                "{err:?}"
            );
        }
        assert!(f.web_sessions.sessions.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn with_a_second_factor_no_session_opens_without_the_code() {
        let f = AdminFixture::new();
        f.with_totp().await;

        let outcome = use_case(&f)
            .execute(command(EMAIL, PASSWORD))
            .await
            .unwrap();
        assert!(matches!(outcome, Outcome::SecondFactorRequired { .. }));
        assert!(f.web_sessions.sessions.lock().unwrap().is_empty());

        let with_code = Command {
            mfa_code: Some(f.totp_code()),
            ..command(EMAIL, PASSWORD)
        };
        started(use_case(&f).execute(with_code).await.unwrap());
        assert_eq!(f.web_sessions.sessions.lock().unwrap().len(), 1);
    }
}
