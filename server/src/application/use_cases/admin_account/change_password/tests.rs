#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::admin_account::change_password::{
            command::Command, use_case::UseCase,
        },
        domain::entities::admin_user::AdminUser,
        testing::in_memory_repositories::in_memory_admin_user_repository::InMemoryAdminUserRepository,
        utils::password_hasher::{compare_password, hash_password},
    };

    const PEPPER: &str = "test-pepper";
    const CURRENT: &str = "current-password";

    fn setup() -> (Arc<InMemoryAdminUserRepository>, uuid::Uuid) {
        let repo = Arc::new(InMemoryAdminUserRepository::new());
        let hash = hash_password(CURRENT, PEPPER).unwrap();
        let admin = AdminUser::new(
            uuid::Uuid::now_v7(),
            "admin@example.com".to_string(),
            "Ada Admin".to_string(),
            hash,
            "argon2id".to_string(),
        );
        let id = admin.id();
        repo.admins.lock().unwrap().push(admin);
        (repo, id)
    }

    #[tokio::test]
    async fn changes_the_password_when_current_is_correct() {
        let (repo, id) = setup();

        UseCase::new(
            repo.clone(),
            PEPPER.to_string(),
            crate::testing::password_hasher::test_password_hasher(),
        )
        .execute(Command {
            admin_id: id,
            current_password: CURRENT.to_string(),
            new_password: "a-brand-new-password".to_string(),
        })
        .await
        .unwrap();

        let stored = &repo.admins.lock().unwrap()[0];
        assert!(compare_password(
            stored.password_hash(),
            "a-brand-new-password",
            PEPPER
        ));
        assert!(!compare_password(stored.password_hash(), CURRENT, PEPPER));
    }

    #[tokio::test]
    async fn rejects_a_wrong_current_password() {
        let (repo, id) = setup();

        let result = UseCase::new(
            repo.clone(),
            PEPPER.to_string(),
            crate::testing::password_hasher::test_password_hasher(),
        )
        .execute(Command {
            admin_id: id,
            current_password: "not-the-password".to_string(),
            new_password: "a-brand-new-password".to_string(),
        })
        .await;

        assert!(result.is_err());
        // Password unchanged.
        let stored = &repo.admins.lock().unwrap()[0];
        assert!(compare_password(stored.password_hash(), CURRENT, PEPPER));
    }
}
