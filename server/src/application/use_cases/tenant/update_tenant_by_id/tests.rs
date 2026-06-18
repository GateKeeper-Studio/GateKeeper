#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant::update_tenant_by_id::{
            command::Command, use_case::UseCase,
        },
        domain::{entities::tenant::Tenant, errors::DomainError},
        testing::{
            in_memory_repositories::{
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn make_tenant() -> Tenant {
        Tenant::new(
            "Original Name".to_string(),
            Some("Original description".to_string()),
            "test_secret".to_string(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
            false,
        )
    }

    fn setup() -> (Arc<InMemoryTenantRepository>, UseCase) {
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let uow_factory = Arc::new(InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            tenant_repo.clone(),
        ));
        let use_case = UseCase::new(uow_factory);
        (tenant_repo, use_case)
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn updates_tenant_successfully() {
        let (tenant_repo, use_case) = setup();
        let tenant = make_tenant();
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let response = use_case
            .execute(Command {
                tenant_id: tenant.id(),
                name: "Updated Name".to_string(),
                description: Some("Updated description".to_string()),
                is_active: false,
                can_self_register: false,
                can_self_forgot_password: false,
            })
            .await
            .unwrap();

        assert_eq!(response.tenant_id, tenant.id());
        assert_eq!(response.name, "Updated Name");
        assert_eq!(
            response.description,
            Some("Updated description".to_string())
        );
        assert!(!response.is_active);
        assert!(!response.can_self_register);
        assert!(!response.can_self_forgot_password);
        assert!(response.updated_at.is_some());
    }

    #[tokio::test]
    async fn persists_update_to_repository() {
        let (tenant_repo, use_case) = setup();
        let tenant = make_tenant();
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        use_case
            .execute(Command {
                tenant_id: tenant.id(),
                name: "Persisted Name".to_string(),
                description: None,
                is_active: true,
                can_self_register: false,
                can_self_forgot_password: true,
            })
            .await
            .unwrap();

        let saved = tenant_repo.memory.data.lock().unwrap();
        let saved_tenant = saved.get(&tenant.id()).unwrap();
        assert_eq!(saved_tenant.name(), "Persisted Name");
        assert_eq!(saved_tenant.description(), None);
        assert!(saved_tenant.updated_at().is_some());
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let (_, use_case) = setup();

        let err = use_case
            .execute(Command {
                tenant_id: Uuid::new_v4(),
                name: "Any Name".to_string(),
                description: None,
                is_active: true,
                can_self_register: false,
                can_self_forgot_password: false,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TenantNotFound(_)));
    }
}
