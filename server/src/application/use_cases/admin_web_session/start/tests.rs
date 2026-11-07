#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::admin_web_session::start::{command::Command, use_case::UseCase},
        domain::{entities::admin_user::AdminUser, errors::DomainError},
        testing::{
            in_memory_repositories::{
                in_memory_admin_user_repository::InMemoryAdminUserRepository,
                in_memory_admin_web_session_repository::InMemoryAdminWebSessionRepository,
            },
            password_hasher::test_password_hasher,
        },
        utils::{oauth_crypto::sha256_hex, password_hasher::hash_password},
    };

    const PEPPER: &str = "test-server-wide-pepper";
    const EMAIL: &str = "jane.doe@example.com";
    const PASSWORD: &str = "Sup3rSecret!";

    fn setup(name: &str) -> (UseCase, Arc<InMemoryAdminWebSessionRepository>, AdminUser) {
        let admins = Arc::new(InMemoryAdminUserRepository::new());
        let sessions = Arc::new(InMemoryAdminWebSessionRepository::new());
        let admin = AdminUser::new(
            uuid::Uuid::now_v7(),
            EMAIL.to_string(),
            name.to_string(),
            hash_password(PASSWORD, PEPPER).unwrap(),
            "argon2id".to_string(),
        );
        admins.admins.lock().unwrap().push(admin.clone());
        let use_case = UseCase::new(
            admins,
            sessions.clone(),
            PEPPER.to_string(),
            test_password_hasher(),
        );
        (use_case, sessions, admin)
    }

    fn command(email: &str, password: &str) -> Command {
        Command {
            email: email.to_string(),
            password: password.to_string(),
        }
    }

    #[tokio::test]
    async fn signing_in_stores_only_the_hash_of_the_session_id() {
        let (use_case, sessions, admin) = setup("Ada Admin");

        let response = use_case.execute(command(EMAIL, PASSWORD)).await.unwrap();

        let stored = sessions.sessions.lock().unwrap().clone();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].admin_id(), admin.id());
        assert_eq!(stored[0].token_hash(), sha256_hex(&response.session_token));
        assert_ne!(stored[0].token_hash(), response.session_token);
        assert_eq!(response.admin.name, "Ada Admin");
        assert_eq!(response.admin.initials, "AA");
        assert_eq!(
            response.admin.organization_id,
            admin.organization_id().to_string()
        );
    }

    #[tokio::test]
    async fn an_empty_name_falls_back_to_the_email() {
        let (use_case, _, _) = setup("  ");
        let response = use_case.execute(command(EMAIL, PASSWORD)).await.unwrap();
        assert_eq!(response.admin.name, "Jane Doe");
    }

    #[tokio::test]
    async fn wrong_password_and_unknown_email_fail_the_same_way_and_store_nothing() {
        let (use_case, sessions, _) = setup("Ada");

        for (email, password) in [(EMAIL, "wrong"), ("nobody@example.com", PASSWORD)] {
            let err = use_case
                .execute(command(email, password))
                .await
                .unwrap_err();
            assert!(
                matches!(&err, DomainError::AuthenticationError(m) if m == "invalid credentials"),
                "{err:?}"
            );
        }
        assert!(sessions.sessions.lock().unwrap().is_empty());
    }
}
