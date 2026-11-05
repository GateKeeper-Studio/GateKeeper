#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant_custom_field::update_tenant_custom_field_by_id::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::custom_field_definition::{CustomFieldDefinition, CustomFieldType},
            entities::tenant::Tenant,
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_custom_field_definition_repository::InMemoryCustomFieldDefinitionRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn locale_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn setup() -> (
        Arc<InMemoryCustomFieldDefinitionRepository>,
        Uuid,
        Uuid,
        UseCase,
    ) {
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
        let def = CustomFieldDefinition::new(
            tenant_id,
            "size".to_string(),
            locale_map(&[("en", "Size")]),
            None,
            CustomFieldType::Select,
            Some(vec![locale_map(&[("en", "S")]), locale_map(&[("en", "M")])]),
            false,
            0,
        );
        let field_id = def.id();
        fields.memory.data.lock().unwrap().insert(field_id, def);

        let factory =
            InMemoryUnitOfWorkFactory::new(Arc::new(InMemoryUserRepository::new()), tenant_repo)
                .with_custom_field_definition_repository(fields.clone());

        let use_case = UseCase::new(Arc::new(factory));
        (fields, tenant_id, field_id, use_case)
    }

    fn make_command(tenant_id: Uuid, field_id: Uuid) -> Command {
        Command {
            tenant_id,
            field_id,
            label: locale_map(&[("en", "Shirt size")]),
            description: Some(locale_map(&[("en", "Pick your size")])),
            options: Some(vec![
                locale_map(&[("en", "S")]),
                locale_map(&[("en", "M")]),
                locale_map(&[("en", "L")]),
            ]),
            is_required: true,
            display_order: 1,
            is_active: true,
        }
    }

    #[tokio::test]
    async fn updates_label_options_and_flags() {
        let (fields, tenant_id, field_id, use_case) = setup();

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant_id, field_id),
            )
            .await
            .unwrap();

        assert_eq!(
            response.label.get("en").map(String::as_str),
            Some("Shirt size")
        );
        assert_eq!(response.options.unwrap().len(), 3);
        assert!(response.is_required);

        // field_key and field_type are untouched.
        let stored = fields.memory.data.lock().unwrap();
        let stored = stored.get(&field_id).unwrap();
        assert_eq!(stored.field_key(), "size");
        assert_eq!(stored.field_type(), CustomFieldType::Select);
    }

    #[tokio::test]
    async fn fails_when_field_not_found() {
        let (_, tenant_id, _, use_case) = setup();
        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(tenant_id, Uuid::now_v7()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let (_, _, field_id, use_case) = setup();
        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command(Uuid::now_v7(), field_id),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn rejects_empty_options_for_select_field() {
        let (_, tenant_id, field_id, use_case) = setup();
        let mut command = make_command(tenant_id, field_id);
        command.options = None;

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_label_missing_tenant_default_language() {
        let (_, tenant_id, field_id, use_case) = setup();
        let mut command = make_command(tenant_id, field_id);
        command.label = locale_map(&[("es", "Talla")]);

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_unsupported_locale_key() {
        let (_, tenant_id, field_id, use_case) = setup();
        let mut command = make_command(tenant_id, field_id);
        command.label = locale_map(&[("en", "Shirt size"), ("fr", "Taille")]);

        let err = use_case
            .execute(&crate::application::shared::actor::Actor::System, command)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }
}
