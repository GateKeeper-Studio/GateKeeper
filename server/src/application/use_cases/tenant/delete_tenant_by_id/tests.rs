#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant::delete_tenant_by_id::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::tenant::Tenant, errors::DomainError,
            repositories::tenant_repository::TenantRepository,
        },
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
            "Test Tenant".to_string(),
            None,
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
    async fn deletes_tenant_successfully() {
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
            })
            .await
            .unwrap();

        let found = tenant_repo.find_by_id(tenant.id()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let (_, use_case) = setup();

        let err = use_case
            .execute(Command {
                tenant_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TenantNotFound(_)));
    }
}
