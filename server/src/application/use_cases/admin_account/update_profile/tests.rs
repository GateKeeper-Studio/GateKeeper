#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::admin_account::update_profile::{
            command::Command, use_case::UseCase,
        },
        domain::entities::admin_user::AdminUser,
        testing::in_memory_repositories::in_memory_admin_user_repository::InMemoryAdminUserRepository,
    };

    fn admin(email: &str, name: &str) -> AdminUser {
        AdminUser::new(
            uuid::Uuid::now_v7(),
            email.to_string(),
            name.to_string(),
            "hash".to_string(),
            "argon2id".to_string(),
        )
    }

    #[tokio::test]
    async fn updates_name_and_email() {
        let repo = Arc::new(InMemoryAdminUserRepository::new());
        let seeded = admin("old@example.com", "Old Name");
        let id = seeded.id();
        repo.admins.lock().unwrap().push(seeded);

        let response = UseCase::new(repo.clone())
            .execute(Command {
                admin_id: id,
                name: "New Name".to_string(),
                email: "new@example.com".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(response.name, "New Name");
        assert_eq!(response.email, "new@example.com");
        assert!(response.updated_at.is_some());

        let stored = &repo.admins.lock().unwrap()[0];
        assert_eq!(stored.email(), "new@example.com");
        assert_eq!(stored.name(), "New Name");
    }

    #[tokio::test]
    async fn rejects_an_email_already_taken_by_another_admin() {
        let repo = Arc::new(InMemoryAdminUserRepository::new());
        let other = admin("taken@example.com", "Other");
        let me = admin("me@example.com", "Me");
        let my_id = me.id();
        repo.admins.lock().unwrap().push(other);
        repo.admins.lock().unwrap().push(me);

        let result = UseCase::new(repo)
            .execute(Command {
                admin_id: my_id,
                name: "Me".to_string(),
                email: "taken@example.com".to_string(),
            })
            .await;

        assert!(result.is_err());
    }
}
