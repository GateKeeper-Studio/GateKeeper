#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use uuid::Uuid;

    fn en(value: &str) -> HashMap<String, String> {
        HashMap::from([("en".to_string(), value.to_string())])
    }

    use crate::{
        application::use_cases::tenant_custom_field::delete_tenant_custom_field_by_id::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::custom_field_definition::{CustomFieldDefinition, CustomFieldType},
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

    #[tokio::test]
    async fn deletes_the_field_definition() {
        let tenant_id = Uuid::now_v7();
        let fields = Arc::new(InMemoryCustomFieldDefinitionRepository::new());
        let def = CustomFieldDefinition::new(
            tenant_id,
            "nickname".to_string(),
            en("Nickname"),
            None,
            CustomFieldType::Text,
            None,
            false,
            0,
        );
        let field_id = def.id();
        fields.memory.data.lock().unwrap().insert(field_id, def);

        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_custom_field_definition_repository(fields.clone());
        let use_case = UseCase::new(Arc::new(factory));

        use_case
            .execute(Command {
                tenant_id,
                field_id,
            })
            .await
            .unwrap();

        // commit() drains `.deleted` into a removal from `.data`, so the row is
        // simply gone from `.data` afterward (not left sitting in `.deleted`).
        assert!(!fields.memory.data.lock().unwrap().contains_key(&field_id));
    }

    #[tokio::test]
    async fn fails_when_field_not_found() {
        let fields = Arc::new(InMemoryCustomFieldDefinitionRepository::new());
        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_custom_field_definition_repository(fields);
        let use_case = UseCase::new(Arc::new(factory));

        let err = use_case
            .execute(Command {
                tenant_id: Uuid::now_v7(),
                field_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::CustomFieldNotFound(_)));
    }
}
