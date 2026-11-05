#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::compliance::list_user_acceptances::use_case::UseCase,
        domain::{
            entities::{policy_acceptance::PolicyAcceptance, user::User},
            errors::DomainError,
            repositories::policy_acceptance_repository::PolicyAcceptanceRepository,
        },
        testing::in_memory_repositories::{
            in_memory_policy_acceptance_repository::InMemoryPolicyAcceptanceRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
    };

    fn seeded_user(tenant_id: Uuid, user_id: Uuid) -> User {
        User::reconstruct(
            user_id,
            "ada@example.com".into(),
            tenant_id,
            true,
            None,
            None,
            true,
            Utc::now(),
            None,
        )
    }

    async fn seed_acceptances(
        repo: &InMemoryPolicyAcceptanceRepository,
        tenant_id: Uuid,
        user_id: Uuid,
    ) {
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
    }

    #[tokio::test]
    async fn lists_user_acceptances() {
        let repo = Arc::new(InMemoryPolicyAcceptanceRepository::new());
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        seed_acceptances(&repo, tenant_id, user_id).await;

        let users = Arc::new(InMemoryUserRepository::new());
        users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user_id, seeded_user(tenant_id, user_id));

        let use_case = UseCase::new(repo.clone(), users);
        let rows = use_case.execute(tenant_id, user_id).await.unwrap();
        assert_eq!(rows.len(), 2);
    }

    // Tenant isolation: pairing an authorized tenant with a user that belongs to a
    // different tenant must not leak that user's acceptance history.
    #[tokio::test]
    async fn refuses_a_user_from_another_tenant() {
        let repo = Arc::new(InMemoryPolicyAcceptanceRepository::new());
        let user_tenant = Uuid::now_v7();
        let other_tenant = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        seed_acceptances(&repo, user_tenant, user_id).await;

        let users = Arc::new(InMemoryUserRepository::new());
        users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user_id, seeded_user(user_tenant, user_id));

        let use_case = UseCase::new(repo.clone(), users);
        let result = use_case.execute(other_tenant, user_id).await;
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }
}
