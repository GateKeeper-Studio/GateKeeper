#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::admin_account::get_profile::{query::Query, use_case::UseCase},
        domain::entities::admin_user::AdminUser,
        testing::in_memory_repositories::in_memory_admin_user_repository::InMemoryAdminUserRepository,
    };

    fn seed(repo: &InMemoryAdminUserRepository) -> uuid::Uuid {
        let admin = AdminUser::new(
            uuid::Uuid::now_v7(),
            "admin@example.com".to_string(),
            "Ada Admin".to_string(),
            "hash".to_string(),
            "argon2id".to_string(),
        );
        let id = admin.id();
        repo.admins.lock().unwrap().push(admin);
        id
    }

    #[tokio::test]
    async fn returns_the_admin_profile() {
        let repo = Arc::new(InMemoryAdminUserRepository::new());
        let id = seed(&repo);

        let response = UseCase::new(repo)
            .execute(Query { admin_id: id })
            .await
            .unwrap();

        assert_eq!(response.id, id);
        assert_eq!(response.email, "admin@example.com");
        assert_eq!(response.name, "Ada Admin");
        assert!(response.is_active);
    }

    #[tokio::test]
    async fn errors_when_admin_missing() {
        let repo = Arc::new(InMemoryAdminUserRepository::new());

        let result = UseCase::new(repo)
            .execute(Query {
                admin_id: uuid::Uuid::now_v7(),
            })
            .await;

        assert!(result.is_err());
    }
}
