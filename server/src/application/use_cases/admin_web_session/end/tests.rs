#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};

    use crate::{
        application::use_cases::admin_web_session::end::{command::Command, use_case::UseCase},
        domain::entities::admin_web_session::AdminWebSession,
        testing::in_memory_repositories::in_memory_admin_web_session_repository::InMemoryAdminWebSessionRepository,
        utils::oauth_crypto::sha256_hex,
    };

    #[tokio::test]
    async fn signing_out_deletes_only_that_session_and_is_idempotent() {
        let sessions = Arc::new(InMemoryAdminWebSessionRepository::new());
        let admin_id = uuid::Uuid::now_v7();
        for token in ["mine", "other-browser"] {
            sessions.sessions.lock().unwrap().push(AdminWebSession::new(
                admin_id,
                sha256_hex(token),
                Utc::now(),
                Duration::hours(1),
            ));
        }
        let use_case = UseCase::new(sessions.clone());
        let command = || Command {
            session_token: "mine".to_string(),
        };

        use_case.execute(command()).await.unwrap();
        use_case.execute(command()).await.unwrap();

        let left = sessions.sessions.lock().unwrap().clone();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].token_hash(), sha256_hex("other-browser"));
    }
}
