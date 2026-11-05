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
        let tenant = crate::domain::entities::tenant::Tenant::new(
            "Acme".into(),
            None,
            "pepper".into(),
            true,
            true,
            true,
            Uuid::now_v7(),
        );
        let tenant_id = tenant.id();
        let user_id = Uuid::now_v7();
        let tenants = Arc::new(InMemoryTenantRepository::new());
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant_id, tenant);

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

        let factory = InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_consent_grant_repository(consents.clone());

        let use_case = UseCase::new(Arc::new(factory));
        (tenant_id, user_id, users, consents, use_case)
    }

    #[tokio::test]
    async fn anonymizes_user_and_clears_consents() {
        let (tenant_id, user_id, users, consents, use_case) = setup();

        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command { tenant_id, user_id },
            )
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
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command { tenant_id, user_id },
            )
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
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: other_tenant,
                    user_id,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn another_user_cannot_erase_this_account() {
        // The rule is enforced in the use case, not only by the HTTP guard.
        let (tenant_id, user_id, _, _, use_case) = setup();
        let actor = crate::application::shared::actor::Actor::User {
            tenant_id,
            user_id: Uuid::now_v7(),
        };
        let err = use_case
            .execute(&actor, Command { tenant_id, user_id })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)));
    }

    #[tokio::test]
    async fn an_admin_of_another_tenant_cannot_erase_this_account() {
        let (tenant_id, user_id, _, _, use_case) = setup();
        let actor = crate::application::shared::actor::Actor::TenantAdmin {
            tenant_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        };
        let err = use_case
            .execute(&actor, Command { tenant_id, user_id })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)));
    }
}
