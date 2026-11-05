#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use uuid::Uuid;

    fn en(value: &str) -> HashMap<String, String> {
        HashMap::from([("en".to_string(), value.to_string())])
    }

    use crate::{
        application::use_cases::tenant_custom_field::reorder_tenant_custom_fields::{
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

    fn seed_fields(
        tenant_id: Uuid,
        count: usize,
    ) -> (Arc<InMemoryCustomFieldDefinitionRepository>, Vec<Uuid>) {
        let fields = Arc::new(InMemoryCustomFieldDefinitionRepository::new());
        let mut ids = Vec::new();
        for i in 0..count {
            let def = CustomFieldDefinition::new(
                tenant_id,
                format!("field_{i}"),
                en(&format!("Field {i}")),
                None,
                CustomFieldType::Text,
                None,
                false,
                i as i32,
            );
            ids.push(def.id());
            fields.memory.data.lock().unwrap().insert(def.id(), def);
        }
        (fields, ids)
    }

    fn use_case_for(fields: Arc<InMemoryCustomFieldDefinitionRepository>) -> UseCase {
        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_custom_field_definition_repository(fields);
        UseCase::new(Arc::new(factory))
    }

    #[tokio::test]
    async fn reorders_fields_to_match_the_submitted_order() {
        let tenant_id = Uuid::now_v7();
        let (fields, ids) = seed_fields(tenant_id, 3);
        let new_order = vec![ids[2], ids[0], ids[1]];
        let use_case = use_case_for(fields.clone());

        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    field_ids: new_order.clone(),
                },
            )
            .await
            .unwrap();

        let data = fields.memory.data.lock().unwrap();
        for (index, id) in new_order.iter().enumerate() {
            assert_eq!(data.get(id).unwrap().display_order(), index as i32);
        }
    }

    #[tokio::test]
    async fn rejects_a_list_missing_an_existing_id() {
        let tenant_id = Uuid::now_v7();
        let (fields, ids) = seed_fields(tenant_id, 3);
        let use_case = use_case_for(fields.clone());

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    field_ids: vec![ids[0], ids[1]],
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Validation(_)));
        let data = fields.memory.data.lock().unwrap();
        for (index, id) in ids.iter().enumerate() {
            assert_eq!(data.get(id).unwrap().display_order(), index as i32);
        }
    }

    #[tokio::test]
    async fn rejects_a_list_with_an_unknown_id() {
        let tenant_id = Uuid::now_v7();
        let (fields, ids) = seed_fields(tenant_id, 2);
        let use_case = use_case_for(fields);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    field_ids: vec![ids[0], ids[1], Uuid::now_v7()],
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn rejects_a_list_with_a_duplicate_id() {
        let tenant_id = Uuid::now_v7();
        let (fields, ids) = seed_fields(tenant_id, 2);
        let use_case = use_case_for(fields);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    field_ids: vec![ids[0], ids[0]],
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Validation(_)));
    }
}
