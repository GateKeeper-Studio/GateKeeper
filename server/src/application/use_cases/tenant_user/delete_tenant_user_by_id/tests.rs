#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant_user::delete_tenant_user_by_id::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::tenant::Tenant, errors::DomainError,
            repositories::user_repository::UserRepository,
        },
        testing::{
            in_memory_repositories::{
                in_memory_outbox_event_repository::InMemoryOutboxEventRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            user_factory::generate_user,
        },
    };

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn make_tenant() -> Tenant {
        Tenant::new(
            "Test Tenant".to_string(), // name
            None,                      // description
            "test_secret".to_string(), // password_hash_secret
            true,                      // is_active
            true,                      // can_self_register
            true,                      // can_self_forgot_password
            uuid::Uuid::now_v7(),      // organization_id
        )
    }

    fn setup(tenant: &Tenant) -> (Arc<InMemoryUserRepository>, UseCase) {
        let (user_repo, use_case, _) = setup_with_outbox(tenant);
        (user_repo, use_case)
    }

    fn setup_with_outbox(
        tenant: &Tenant,
    ) -> (
        Arc<InMemoryUserRepository>,
        UseCase,
        Arc<InMemoryOutboxEventRepository>,
    ) {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let outbox = Arc::new(InMemoryOutboxEventRepository::new());

        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(user_repo.clone(), tenant_repo.clone())
                .with_outbox_event_repository(outbox.clone()),
        );

        let use_case = UseCase::new(uow_factory);

        (user_repo, use_case, outbox)
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn deletes_user_successfully() {
        let tenant = make_tenant();
        let (user_repo, use_case) = setup(&tenant);
        let user = generate_user(tenant.id());
        user_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());

        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    user_id: user.id(),
                },
            )
            .await
            .unwrap();

        let found = user_repo.find_by_id(user.id()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn fails_when_user_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    user_id: Uuid::new_v4(),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let tenant = make_tenant();
        let (_, use_case) = setup(&tenant);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: Uuid::new_v4(),
                    user_id: Uuid::new_v4(),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_user_belongs_to_different_tenant() {
        let tenant = make_tenant();
        let other_tenant = make_tenant();
        let (user_repo, use_case) = setup(&tenant);

        let user = generate_user(other_tenant.id());
        user_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    user_id: user.id(),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    // ── Outbound events ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn enqueues_a_user_deleted_event() {
        let tenant = make_tenant();
        let (user_repo, use_case, outbox) = setup_with_outbox(&tenant);
        let user = generate_user(tenant.id());
        let user_id = user.id();
        user_repo.memory.data.lock().unwrap().insert(user_id, user);

        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    user_id,
                },
            )
            .await
            .unwrap();

        let stored = outbox.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let event = stored.values().next().unwrap();
        assert_eq!(event.event_type().as_str(), "user.deleted");
        assert_eq!(event.tenant_id(), tenant.id());
        // Only the id: there is no user left to describe.
        assert_eq!(event.payload()["data"]["user_id"], user_id.to_string());
    }

    // Deleting a user who is not there announces nothing.
    #[tokio::test]
    async fn a_failed_delete_leaves_no_event_behind() {
        let tenant = make_tenant();
        let (_, use_case, outbox) = setup_with_outbox(&tenant);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    user_id: Uuid::now_v7(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));

        assert_eq!(outbox.memory.data.lock().unwrap().len(), 0);
    }
}
