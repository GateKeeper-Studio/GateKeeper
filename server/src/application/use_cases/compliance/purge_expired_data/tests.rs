#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::use_cases::compliance::purge_expired_data::use_case::UseCase,
        domain::entities::refresh_token::RefreshToken,
        testing::in_memory_repositories::{
            in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        testing::in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
    };

    #[tokio::test]
    async fn purges_only_expired_rows() {
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let client_id = Uuid::now_v7();

        let refresh = Arc::new(InMemoryRefreshTokenRepository::new());
        let expired = RefreshToken::new(
            tenant_id,
            "h1".into(),
            client_id,
            user_id,
            vec![],
            Utc::now() - Duration::days(1),
        );
        let fresh = RefreshToken::new(
            tenant_id,
            "h2".into(),
            client_id,
            user_id,
            vec![],
            Utc::now() + Duration::days(1),
        );
        {
            let mut data = refresh.memory.data.lock().unwrap();
            data.insert(expired.id(), expired);
            data.insert(fresh.id(), fresh);
        }

        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_refresh_token_repository(refresh.clone());

        let use_case = UseCase::new(Arc::new(factory));
        let report = use_case.execute().await.unwrap();

        assert_eq!(report.refresh_tokens, 1);
        assert_eq!(report.total, 1);
        assert_eq!(refresh.memory.data.lock().unwrap().len(), 1); // only the fresh one remains
    }
}
