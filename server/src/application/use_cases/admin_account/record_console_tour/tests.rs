#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::admin_account::record_console_tour::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::admin_user::AdminUser, errors::DomainError,
            repositories::admin_user_repository::AdminUserRepository,
        },
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

    async fn seen(repo: &InMemoryAdminUserRepository, id: uuid::Uuid) -> i32 {
        repo.find_by_id(id)
            .await
            .unwrap()
            .unwrap()
            .console_tour_version()
    }

    #[tokio::test]
    async fn a_new_admin_has_not_seen_the_tour_and_recording_it_sticks() {
        let repo = Arc::new(InMemoryAdminUserRepository::new());
        let id = seed(&repo);
        assert_eq!(seen(&repo, id).await, 0);

        let use_case = UseCase::new(repo.clone());
        use_case
            .execute(Command {
                admin_id: id,
                version: 1,
            })
            .await
            .unwrap();

        assert_eq!(seen(&repo, id).await, 1);
    }

    #[tokio::test]
    async fn an_older_version_never_lowers_the_one_recorded() {
        let repo = Arc::new(InMemoryAdminUserRepository::new());
        let id = seed(&repo);
        let use_case = UseCase::new(repo.clone());

        for version in [3, 1] {
            use_case
                .execute(Command {
                    admin_id: id,
                    version,
                })
                .await
                .unwrap();
        }

        assert_eq!(seen(&repo, id).await, 3);
    }

    #[tokio::test]
    async fn a_version_out_of_range_is_refused() {
        let repo = Arc::new(InMemoryAdminUserRepository::new());
        let id = seed(&repo);
        let use_case = UseCase::new(repo.clone());

        for version in [0, -1, 1001, i32::MAX] {
            let result = use_case
                .execute(Command {
                    admin_id: id,
                    version,
                })
                .await;
            assert!(matches!(result, Err(DomainError::Validation(_))));
        }
        assert_eq!(seen(&repo, id).await, 0);
    }
}
