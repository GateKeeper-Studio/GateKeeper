#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant::update_tenant_appearance::{
            command::{AppearanceInput, Command},
            use_case::UseCase,
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
            "Acme".to_string(),
            None,
            "test_secret".to_string(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
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

    fn appearance(color: &str, radius: &str, mode: &str) -> Option<AppearanceInput> {
        Some(AppearanceInput {
            primary_color: Some(color.to_string()),
            radius: Some(radius.to_string()),
            default_mode: Some(mode.to_string()),
        })
    }

    // ── Tests ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn sets_and_persists_appearance() {
        let (tenant_repo, use_case) = setup();
        let tenant = make_tenant();
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    organization_id: tenant.organization_id(),
                    appearance: appearance("#7C3AED", "lg", "dark"),
                },
            )
            .await
            .unwrap();

        let stored = response.appearance.expect("appearance present");
        assert_eq!(stored["primaryColor"], "#7C3AED");
        assert_eq!(stored["radius"], "lg");
        assert_eq!(stored["defaultMode"], "dark");
        assert!(response.updated_at.is_some());

        let saved = tenant_repo.memory.data.lock().unwrap();
        let saved_tenant = saved.get(&tenant.id()).unwrap();
        assert_eq!(saved_tenant.appearance().unwrap()["radius"], "lg");
    }

    #[tokio::test]
    async fn null_appearance_clears_theme() {
        let (tenant_repo, use_case) = setup();
        let mut tenant = make_tenant();
        tenant.set_appearance(Some(serde_json::json!({
            "primaryColor": "#000000", "radius": "md", "defaultMode": "system"
        })));
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    organization_id: tenant.organization_id(),
                    appearance: None,
                },
            )
            .await
            .unwrap();

        assert!(response.appearance.is_none());
    }

    #[tokio::test]
    async fn defaults_radius_and_mode_when_omitted() {
        let (tenant_repo, use_case) = setup();
        let tenant = make_tenant();
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    organization_id: tenant.organization_id(),
                    appearance: Some(AppearanceInput {
                        primary_color: Some("#123abc".to_string()),
                        radius: None,
                        default_mode: None,
                    }),
                },
            )
            .await
            .unwrap();

        let stored = response.appearance.unwrap();
        assert_eq!(stored["radius"], "md");
        assert_eq!(stored["defaultMode"], "system");
    }

    #[tokio::test]
    async fn rejects_non_hex_color() {
        let (tenant_repo, use_case) = setup();
        let tenant = make_tenant();
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    organization_id: tenant.organization_id(),
                    appearance: appearance("red; }", "md", "system"),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn rejects_unknown_radius() {
        let (tenant_repo, use_case) = setup();
        let tenant = make_tenant();
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    organization_id: tenant.organization_id(),
                    appearance: appearance("#7C3AED", "gigantic", "system"),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let (_, use_case) = setup();

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: Uuid::new_v4(),
                    organization_id: Uuid::new_v4(),
                    appearance: appearance("#7C3AED", "md", "system"),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
