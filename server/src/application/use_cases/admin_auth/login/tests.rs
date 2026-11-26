#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::admin_auth::login::{
            command::Command,
            response::{Outcome, Response},
            use_case::UseCase,
        },
        domain::errors::DomainError,
        testing::{
            admin_fixture::{AdminFixture, EMAIL, PASSWORD},
            fake_admin_token_issuer::FakeAdminTokenIssuer,
        },
    };

    fn use_case(f: &AdminFixture) -> UseCase {
        UseCase::new(
            f.authenticator.clone(),
            f.refresh_tokens.clone(),
            Arc::new(FakeAdminTokenIssuer::new()),
        )
    }

    fn command(email: &str, password: &str, mfa_code: Option<String>) -> Command {
        Command {
            email: email.to_string(),
            password: password.to_string(),
            mfa_code,
        }
    }

    fn signed_in(outcome: Outcome) -> Response {
        match outcome {
            Outcome::SignedIn { response, .. } => response,
            Outcome::SecondFactorRequired { .. } => panic!("expected a sign-in"),
        }
    }

    #[tokio::test]
    async fn issues_a_token_on_valid_credentials() {
        let f = AdminFixture::new();

        let response = signed_in(
            use_case(&f)
                .execute(command(EMAIL, PASSWORD, None))
                .await
                .unwrap(),
        );

        assert_eq!(response.token_type, "Bearer");
        assert!(response.expires_in > 0);
        assert!(response.access_token.contains(&f.admin.id().to_string()));

        // A refresh token is returned (raw) and persisted (hashed, so it never equals raw).
        assert!(!response.refresh_token.is_empty());
        let stored = f.refresh_tokens.tokens.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].admin_id(), f.admin.id());
        assert_ne!(stored[0].token_hash(), response.refresh_token);
    }

    #[tokio::test]
    async fn rejects_wrong_password_and_unknown_email_alike() {
        let f = AdminFixture::new();
        for (email, password) in [(EMAIL, "wrong-password"), ("nobody@example.com", PASSWORD)] {
            let err = use_case(&f)
                .execute(command(email, password, None))
                .await
                .err()
                .unwrap();
            assert!(matches!(err, DomainError::AuthenticationError(_)));
        }
        assert!(f.refresh_tokens.tokens.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn with_a_second_factor_no_token_is_issued_until_the_code_is_sent() {
        let f = AdminFixture::new();
        f.with_totp().await;

        let first = use_case(&f)
            .execute(command(EMAIL, PASSWORD, None))
            .await
            .unwrap();
        assert!(
            matches!(first, Outcome::SecondFactorRequired { admin_id } if admin_id == f.admin.id())
        );
        assert!(f.refresh_tokens.tokens.lock().unwrap().is_empty());

        let response = signed_in(
            use_case(&f)
                .execute(command(EMAIL, PASSWORD, Some(f.totp_code())))
                .await
                .unwrap(),
        );
        assert!(!response.access_token.is_empty());
    }
}
