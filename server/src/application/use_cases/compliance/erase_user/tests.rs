#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::compliance::erase_user::{command::Command, use_case::UseCase},
        domain::{
            entities::{consent_grant::ConsentGrant, user::User, user_profile::UserProfile},
            errors::DomainError,
            repositories::{
                consent_grant_repository::ConsentGrantRepository, user_repository::UserRepository,
            },
        },
        testing::in_memory_repositories::{
            in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        testing::in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
    };

    fn seeded_user(tenant_id: Uuid, user_id: Uuid) -> User {
        let profile = UserProfile::new(
            user_id,
            Some("Ada L.".into()),
            "Ada".into(),
            "Lovelace".into(),
        );
        User::reconstruct(
            user_id,
            "ada@example.com".into(),
            tenant_id,
            true,
            Some(profile),
            None,
            true,
            Utc::now(),
            None,
        )
    }

    fn setup() -> (
        Uuid,
        Uuid,
        Arc<InMemoryUserRepository>,
        Arc<InMemoryConsentGrantRepository>,
        UseCase,
    ) {
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let users = Arc::new(InMemoryUserRepository::new());
        users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user_id, seeded_user(tenant_id, user_id));

        let consents = Arc::new(InMemoryConsentGrantRepository::new());
        let grant = ConsentGrant::new(tenant_id, user_id, Uuid::now_v7(), vec!["openid".into()]);
        consents
            .memory
            .data
            .lock()
            .unwrap()
            .insert(grant.id(), grant);

        let factory = InMemoryUnitOfWorkFactory::new(
            users.clone(),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_consent_grant_repository(consents.clone());

        let use_case = UseCase::new(Arc::new(factory));
        (tenant_id, user_id, users, consents, use_case)
    }

    #[tokio::test]
    async fn anonymizes_user_and_clears_consents() {
        let (tenant_id, user_id, users, consents, use_case) = setup();

        use_case
            .execute(Command { tenant_id, user_id })
            .await
            .unwrap();

        let user = users.find_by_id(user_id).await.unwrap().unwrap();
        assert!(user.email().starts_with("deleted+"));
        assert!(user.email().ends_with("@anonymized.invalid"));
        assert!(!user.is_active());
        assert!(user.profile().is_none());

        assert!(consents.find_by_user(user_id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn legal_hold_blocks_erasure() {
        let (tenant_id, user_id, users, _consents, use_case) = setup();
        users.legal_holds.lock().unwrap().insert(user_id);

        let err = use_case
            .execute(Command { tenant_id, user_id })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)));

        // Still intact.
        let user = users.find_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(user.email(), "ada@example.com");
    }

    #[tokio::test]
    async fn rejects_user_from_another_tenant() {
        let (_tenant_id, user_id, _users, _consents, use_case) = setup();
        let other_tenant = Uuid::now_v7();
        let err = use_case
            .execute(Command {
                tenant_id: other_tenant,
                user_id,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::UserNotFound(_)));
    }
}
