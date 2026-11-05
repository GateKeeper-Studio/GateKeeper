#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use uuid::Uuid;

    fn en(value: &str) -> HashMap<String, String> {
        HashMap::from([("en".to_string(), value.to_string())])
    }

    use crate::{
        application::use_cases::tenant_custom_field::get_tenant_custom_field_by_id::{
            query::Query, use_case::UseCase,
        },
        domain::{
            entities::custom_field_definition::{CustomFieldDefinition, CustomFieldType},
            errors::DomainError,
        },
        testing::in_memory_repositories::in_memory_custom_field_definition_repository::InMemoryCustomFieldDefinitionRepository,
    };

    #[tokio::test]
    async fn returns_the_field_definition() {
        let repo = Arc::new(InMemoryCustomFieldDefinitionRepository::new());
        let tenant_id = Uuid::now_v7();
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
        repo.memory.data.lock().unwrap().insert(field_id, def);

        let use_case = UseCase::new(repo);
        let response = use_case
            .execute(Query {
                tenant_id,
                field_id,
            })
            .await
            .unwrap();

        assert_eq!(response.field_key, "nickname");
    }

    #[tokio::test]
    async fn fails_when_not_found() {
        let repo = Arc::new(InMemoryCustomFieldDefinitionRepository::new());
        let use_case = UseCase::new(repo);

        let err = use_case
            .execute(Query {
                tenant_id: Uuid::now_v7(),
                field_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
