#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::compliance::list_user_acceptances::use_case::UseCase,
        domain::{
            entities::policy_acceptance::PolicyAcceptance,
            repositories::policy_acceptance_repository::PolicyAcceptanceRepository,
        },
        testing::in_memory_repositories::in_memory_policy_acceptance_repository::InMemoryPolicyAcceptanceRepository,
    };

    #[tokio::test]
    async fn lists_user_acceptances() {
        let repo = Arc::new(InMemoryPolicyAcceptanceRepository::new());
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        repo.record(&PolicyAcceptance::new(
            tenant_id,
            user_id,
            "terms_of_service".into(),
            "1".into(),
            None,
        ))
        .await
        .unwrap();
        repo.record(&PolicyAcceptance::new(
            tenant_id,
            user_id,
            "privacy_policy".into(),
            "1".into(),
            None,
        ))
        .await
        .unwrap();
        repo.commit_acceptances();

        let use_case = UseCase::new(repo.clone());
        let rows = use_case.execute(user_id).await.unwrap();
        assert_eq!(rows.len(), 2);
    }
}
