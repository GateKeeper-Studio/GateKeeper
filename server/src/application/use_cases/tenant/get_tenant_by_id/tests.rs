#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant::get_tenant_by_id::{query::Query, use_case::UseCase},
        domain::{entities::tenant::Tenant, errors::DomainError},
        testing::in_memory_repositories::in_memory_tenant_repository::InMemoryTenantRepository,
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
        let use_case = UseCase::new(tenant_repo.clone());
        (tenant_repo, use_case)
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn returns_tenant_when_found() {
        let (tenant_repo, use_case) = setup();
        let tenant = make_tenant();
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
            })
            .await
            .unwrap();

        assert_eq!(response.tenant_id, tenant.id());
        assert_eq!(response.name, tenant.name());
        assert_eq!(response.is_active, tenant.is_active());
    }

    #[tokio::test]
    async fn returns_description_and_flags_correctly() {
        let (tenant_repo, use_case) = setup();
        let tenant = Tenant::new(
            "Detailed".to_string(),
            Some("A detailed description".to_string()),
            "secret".to_string(),
            false,
            true,
            false,
            uuid::Uuid::now_v7(),
            false,
        );
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let response = use_case
            .execute(Query {
                tenant_id: tenant.id(),
            })
            .await
            .unwrap();

        assert_eq!(
            response.description,
            Some("A detailed description".to_string())
        );
        assert!(!response.is_active);
        assert!(response.can_self_register);
        assert!(!response.can_self_forgot_password);
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let (_, use_case) = setup();

        let err = use_case
            .execute(Query {
                tenant_id: Uuid::new_v4(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::TenantNotFound(_)));
    }
}
