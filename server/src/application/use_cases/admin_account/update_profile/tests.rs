#[cfg(test)]
mod tests {
    use crate::{
        application::use_cases::admin_account::update_profile::{
            command::Command, use_case::UseCase,
        },
        domain::errors::DomainError,
        testing::admin_fixture::{AdminFixture, EMAIL, PASSWORD},
    };

    fn use_case(f: &AdminFixture) -> UseCase {
        UseCase::new(f.admins.clone(), f.authenticator.clone())
    }

    fn command(f: &AdminFixture, name: &str, email: &str, password: Option<&str>) -> Command {
        Command {
            admin_id: f.admin.id(),
            name: name.to_string(),
            email: email.to_string(),
            current_password: password.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn a_name_change_needs_no_password() {
        let f = AdminFixture::new();
        let response = use_case(&f)
            .execute(command(&f, " New Name ", EMAIL, None))
            .await
            .unwrap();
        assert_eq!(response.name, "New Name");
        assert!(response.updated_at.is_some());
        assert_eq!(f.stored().name(), "New Name");
    }

    #[tokio::test]
    async fn an_email_change_needs_the_current_password_and_is_stored_lowercased() {
        let f = AdminFixture::new();
        let refused = use_case(&f)
            .execute(command(&f, "Ada Admin", "New@Example.com", None))
            .await;
        assert!(matches!(refused, Err(DomainError::StepUpRequired(_))));
        let wrong = use_case(&f)
            .execute(command(&f, "Ada Admin", "new@example.com", Some("nope")))
            .await;
        assert!(matches!(wrong, Err(DomainError::Validation(_))));
        assert_eq!(f.stored().email(), EMAIL);

        let response = use_case(&f)
            .execute(command(&f, "Ada Admin", "New@Example.com", Some(PASSWORD)))
            .await
            .unwrap();
        assert_eq!(response.email, "new@example.com");
        assert_eq!(f.stored().email(), "new@example.com");
    }

    #[tokio::test]
    async fn the_same_email_in_another_case_is_not_a_change() {
        let f = AdminFixture::new();
        use_case(&f)
            .execute(command(&f, "Ada Admin", "ADMIN@example.com", None))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn rejects_an_email_already_taken_by_another_admin() {
        let f = AdminFixture::new();
        f.add_colleague("taken@example.com");
        let result = use_case(&f)
            .execute(command(
                &f,
                "Ada Admin",
                "Taken@example.com",
                Some(PASSWORD),
            ))
            .await;
        assert!(matches!(result, Err(DomainError::Conflict(_))));
    }

    #[tokio::test]
    async fn a_blank_name_is_refused() {
        let f = AdminFixture::new();
        let result = use_case(&f).execute(command(&f, "   ", EMAIL, None)).await;
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }
}
