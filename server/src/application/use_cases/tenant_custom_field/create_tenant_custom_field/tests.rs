#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant_custom_field::create_tenant_custom_field::{
            command::Command, use_case::UseCase,
        },
        domain::{entities::tenant::Tenant, errors::DomainError},
        testing::{
            in_memory_repositories::{
                in_memory_custom_field_definition_repository::InMemoryCustomFieldDefinitionRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn setup() -> (Arc<InMemoryCustomFieldDefinitionRepository>, Uuid, UseCase) {
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::new(
            "Test Tenant".to_string(),
            None,
            "secret".to_string(),
            true,
            true,
            true,
            Uuid::now_v7(),
        );
        let tenant_id = tenant.id();
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant_id, tenant);

        let fields = Arc::new(InMemoryCustomFieldDefinitionRepository::new());

        let factory =
            InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenant_repo)
                .with_custom_field_definition_repository(fields.clone());

        let use_case = UseCase::new(Arc::new(factory));
        (fields, tenant_id, use_case)
    }

    fn locale_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn make_command(tenant_id: Uuid) -> Command {
        Command {
            tenant_id,
            field_key: "shirt_size".to_string(),
            label: locale_map(&[("en", "Shirt size")]),
            description: None,
            field_type: "select".to_string(),
            options: Some(vec![
                locale_map(&[("en", "S")]),
                locale_map(&[("en", "M")]),
                locale_map(&[("en", "L")]),
            ]),
            is_required: true,
            display_order: 0,
        }
    }

    #[tokio::test]
    async fn creates_field_definition() {
        let (fields, tenant_id, use_case) = setup();

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant_id),
            )
            .await
            .unwrap();

        assert_eq!(response.field_key, "shirt_size");
        assert_eq!(response.field_type, "select");
        assert_eq!(fields.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn fails_when_tenant_missing() {
        let (_, _, use_case) = setup();
        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(Uuid::now_v7()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_on_duplicate_field_key() {
        let (_, tenant_id, use_case) = setup();
        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant_id),
            )
            .await
            .unwrap();

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant_id),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Conflict(_)));
    }

    #[tokio::test]
    async fn rejects_invalid_field_key_format() {
        let (_, tenant_id, use_case) = setup();
        let mut command = make_command(tenant_id);
        command.field_key = "Not Valid!".to_string();

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_select_without_options() {
        let (_, tenant_id, use_case) = setup();
        let mut command = make_command(tenant_id);
        command.options = None;

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_unknown_field_type() {
        let (_, tenant_id, use_case) = setup();
        let mut command = make_command(tenant_id);
        command.field_type = "richtext".to_string();

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn fails_when_field_limit_reached() {
        let (_, tenant_id, use_case) = setup();
        for i in 0..40 {
            let mut command = make_command(tenant_id);
            command.field_key = format!("field_{i}");
            command.field_type = "text".to_string();
            command.options = None;
            use_case
                .execute(&crate::application::shared::actor::Actor::System, command)
                .await
                .unwrap();
        }

        let mut command = make_command(tenant_id);
        command.field_key = "one_too_many".to_string();
        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_label_missing_tenant_default_language() {
        let (_, tenant_id, use_case) = setup();
        let mut command = make_command(tenant_id);
        // The test tenant's default_language is "en" (Tenant::new's default) —
        // an "es"-only label leaves that required key unfilled.
        command.label = locale_map(&[("es", "Talla")]);

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_unsupported_locale_key_in_label() {
        let (_, tenant_id, use_case) = setup();
        let mut command = make_command(tenant_id);
        command.label = locale_map(&[("en", "Shirt size"), ("fr", "Taille")]);

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_duplicate_options_in_default_locale() {
        let (_, tenant_id, use_case) = setup();
        let mut command = make_command(tenant_id);
        command.options = Some(vec![locale_map(&[("en", "S")]), locale_map(&[("en", "S")])]);

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_option_missing_tenant_default_language() {
        let (_, tenant_id, use_case) = setup();
        let mut command = make_command(tenant_id);
        command.options = Some(vec![locale_map(&[("en", "S")]), locale_map(&[("es", "M")])]);

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }
}
