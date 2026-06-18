#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::compliance::record_policy_acceptance::{
            command::Command, use_case::UseCase,
        },
        domain::repositories::policy_acceptance_repository::PolicyAcceptanceRepository,
        testing::in_memory_repositories::{
            in_memory_policy_acceptance_repository::InMemoryPolicyAcceptanceRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        testing::in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
    };

    fn cmd(tenant_id: Uuid, user_id: Uuid, version: &str) -> Command {
        Command {
            tenant_id,
            user_id,
            document_type: "terms_of_service".into(),
            document_version: version.into(),
            ip_address: Some("203.0.113.1".into()),
        }
    }

    #[tokio::test]
    async fn records_acceptance_and_is_idempotent_on_same_version() {
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let acceptances = Arc::new(InMemoryPolicyAcceptanceRepository::new());
        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_policy_acceptance_repository(acceptances.clone());

        let use_case = UseCase::new(Arc::new(factory));

        use_case
            .execute(cmd(tenant_id, user_id, "1"))
            .await
            .unwrap();
        use_case
            .execute(cmd(tenant_id, user_id, "1"))
            .await
            .unwrap(); // duplicate version

        let stored = acceptances.find_by_user(user_id).await.unwrap();
        assert_eq!(stored.len(), 1); // idempotent

        use_case
            .execute(cmd(tenant_id, user_id, "2"))
            .await
            .unwrap(); // new version
        let stored = acceptances.find_by_user(user_id).await.unwrap();
        assert_eq!(stored.len(), 2);
    }
}
