#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use uuid::Uuid;

    fn en(value: &str) -> HashMap<String, String> {
        HashMap::from([("en".to_string(), value.to_string())])
    }

    use crate::{
        application::use_cases::tenant_custom_field::list_tenant_custom_fields::{
            query::Query, use_case::UseCase,
        },
        domain::entities::custom_field_definition::{CustomFieldDefinition, CustomFieldType},
        testing::in_memory_repositories::in_memory_custom_field_definition_repository::InMemoryCustomFieldDefinitionRepository,
    };

    #[tokio::test]
    async fn lists_fields_ordered_by_display_order() {
        let repo = Arc::new(InMemoryCustomFieldDefinitionRepository::new());
        let tenant_id = Uuid::now_v7();

        let second = CustomFieldDefinition::new(
            tenant_id,
            "second".to_string(),
            en("Second"),
            None,
            CustomFieldType::Text,
            None,
            false,
            1,
        );
        let first = CustomFieldDefinition::new(
            tenant_id,
            "first".to_string(),
            en("First"),
            None,
            CustomFieldType::Text,
            None,
            false,
            0,
        );
        let mut data = repo.memory.data.lock().unwrap();
        data.insert(second.id(), second);
        data.insert(first.id(), first);
        drop(data);

        let use_case = UseCase::new(repo);
        let response = use_case.execute(Query { tenant_id }).await.unwrap();

        assert_eq!(response.fields.len(), 2);
        assert_eq!(response.fields[0].field_key, "first");
        assert_eq!(response.fields[1].field_key, "second");
    }

    #[tokio::test]
    async fn returns_empty_list_for_tenant_with_no_fields() {
        let repo = Arc::new(InMemoryCustomFieldDefinitionRepository::new());
        let use_case = UseCase::new(repo);

        let response = use_case
            .execute(Query {
                tenant_id: Uuid::now_v7(),
            })
            .await
            .unwrap();

        assert!(response.fields.is_empty());
    }
}
