#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};

    use crate::{
        application::use_cases::admin_account::deactivate_account::{
            command::Command, use_case::UseCase,
        },
        domain::entities::{admin_refresh_token::AdminRefreshToken, admin_user::AdminUser},
        testing::in_memory_repositories::{
            in_memory_admin_refresh_token_repository::InMemoryAdminRefreshTokenRepository,
            in_memory_admin_user_repository::InMemoryAdminUserRepository,
        },
    };

    fn admin(org: uuid::Uuid, email: &str) -> AdminUser {
        AdminUser::new(
            org,
            email.to_string(),
            "Ada Admin".to_string(),
            "hash".to_string(),
            "argon2id".to_string(),
        )
    }

    #[tokio::test]
    async fn deactivates_and_revokes_sessions() {
        let org = uuid::Uuid::now_v7();
        let admins = Arc::new(InMemoryAdminUserRepository::new());
        let tokens = Arc::new(InMemoryAdminRefreshTokenRepository::new());

        // Two active admins, so deactivating one is allowed.
        let me = admin(org, "me@example.com");
        let my_id = me.id();
        admins.admins.lock().unwrap().push(me);
        admins
            .admins
            .lock()
            .unwrap()
            .push(admin(org, "peer@example.com"));

        // A live session for the admin being deactivated.
        tokens.tokens.lock().unwrap().push(AdminRefreshToken::new(
            my_id,
            "token-hash".to_string(),
            Utc::now() + Duration::days(30),
        ));

        UseCase::new(admins.clone(), tokens.clone())
            .execute(Command { admin_id: my_id })
            .await
            .unwrap();

        let stored = admins
            .admins
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.id() == my_id)
            .cloned()
            .unwrap();
        assert!(!stored.is_active());
        assert!(tokens.tokens.lock().unwrap()[0].revoked_at().is_some());
    }

    #[tokio::test]
    async fn refuses_to_deactivate_the_last_active_admin() {
        let org = uuid::Uuid::now_v7();
        let admins = Arc::new(InMemoryAdminUserRepository::new());
        let tokens = Arc::new(InMemoryAdminRefreshTokenRepository::new());

        let only = admin(org, "only@example.com");
        let id = only.id();
        admins.admins.lock().unwrap().push(only);

        let result = UseCase::new(admins.clone(), tokens)
            .execute(Command { admin_id: id })
            .await;

        assert!(result.is_err());
        // Still active — the guard blocked the change.
        assert!(admins.admins.lock().unwrap()[0].is_active());
    }
}
