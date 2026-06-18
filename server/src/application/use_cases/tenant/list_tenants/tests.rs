#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::tenant::list_tenants::{query::Query, use_case::UseCase},
        domain::entities::tenant::Tenant,
        testing::in_memory_repositories::in_memory_tenant_repository::InMemoryTenantRepository,
    };

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn setup() -> (Arc<InMemoryTenantRepository>, UseCase) {
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let use_case = UseCase::new(tenant_repo.clone());
        (tenant_repo, use_case)
    }

    fn seed_tenants(repo: &InMemoryTenantRepository, quantity: usize) {
        let mut data = repo.memory.data.lock().unwrap();
        for i in 0..quantity {
            let tenant = Tenant::new(
                format!("Tenant {}", i),
                None,
                format!("secret_{}", i),
                true,
                false,
                false,
                uuid::Uuid::now_v7(),
                false,
            );
            data.insert(tenant.id(), tenant);
        }
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn returns_first_page_with_defaults_when_no_pagination_provided() {
        let (tenant_repo, use_case) = setup();
        seed_tenants(&tenant_repo, 25);

        let response = use_case
            .execute(Query {
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(response.page, 1);
        assert_eq!(response.page_size, 10);
        assert_eq!(response.total, 25);
        assert_eq!(response.items.len(), 10);
    }

    #[tokio::test]
    async fn returns_correct_items_for_requested_page() {
        let (tenant_repo, use_case) = setup();
        seed_tenants(&tenant_repo, 12);

        let response = use_case
            .execute(Query {
                page: Some(2),
                page_size: Some(5),
            })
            .await
            .unwrap();

        assert_eq!(response.page, 2);
        assert_eq!(response.page_size, 5);
        assert_eq!(response.total, 12);
        assert_eq!(response.items.len(), 5);
    }

    #[tokio::test]
    async fn last_page_returns_remaining_items() {
        let (tenant_repo, use_case) = setup();
        seed_tenants(&tenant_repo, 12);

        // 12 tenants, page_size 5: pages 1→5, 2→5, 3→2
        let response = use_case
            .execute(Query {
                page: Some(3),
                page_size: Some(5),
            })
            .await
            .unwrap();

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.total, 12);
    }

    #[tokio::test]
    async fn returns_empty_list_when_no_tenants() {
        let (_, use_case) = setup();

        let response = use_case
            .execute(Query {
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(response.items.len(), 0);
        assert_eq!(response.total, 0);
    }
}
