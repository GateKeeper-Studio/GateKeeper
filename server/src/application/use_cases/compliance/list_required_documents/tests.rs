#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::compliance::list_required_documents::use_case::UseCase,
        domain::{
            entities::policy_acceptance::PolicyAcceptance,
            repositories::policy_acceptance_repository::PolicyAcceptanceRepository,
        },
        testing::in_memory_repositories::in_memory_policy_acceptance_repository::InMemoryPolicyAcceptanceRepository,
    };

    fn required() -> Vec<(String, String)> {
        vec![
            ("terms_of_service".into(), "2".into()),
            ("privacy_policy".into(), "1".into()),
        ]
    }

    #[tokio::test]
    async fn flags_missing_and_outdated_acceptances() {
        let repo = Arc::new(InMemoryPolicyAcceptanceRepository::new());
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        // Accepted ToS v1 (outdated — required is v2); privacy never accepted.
        repo.record(&PolicyAcceptance::new(
            tenant_id,
            user_id,
            "terms_of_service".into(),
            "1".into(),
            None,
        ))
        .await
        .unwrap();
        repo.commit_acceptances();

        let use_case = UseCase::new(repo.clone(), required());
        let rows = use_case.execute(user_id).await.unwrap();

        let tos = rows
            .iter()
            .find(|r| r.document_type == "terms_of_service")
            .unwrap();
        assert!(tos.acceptance_required); // accepted v1, need v2
        assert_eq!(tos.accepted_version.as_deref(), Some("1"));

        let privacy = rows
            .iter()
            .find(|r| r.document_type == "privacy_policy")
            .unwrap();
        assert!(privacy.acceptance_required); // never accepted
        assert_eq!(privacy.accepted_version, None);
    }

    #[tokio::test]
    async fn satisfied_when_current_version_accepted() {
        let repo = Arc::new(InMemoryPolicyAcceptanceRepository::new());
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        repo.record(&PolicyAcceptance::new(
            tenant_id,
            user_id,
            "terms_of_service".into(),
            "2".into(),
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

        let use_case = UseCase::new(repo.clone(), required());
        let rows = use_case.execute(user_id).await.unwrap();
        assert!(rows.iter().all(|r| !r.acceptance_required));
    }
}
