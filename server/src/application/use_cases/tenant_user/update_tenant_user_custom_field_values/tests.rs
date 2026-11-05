#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use serde_json::json;
    use uuid::Uuid;

    fn en(value: &str) -> HashMap<String, String> {
        HashMap::from([("en".to_string(), value.to_string())])
    }

    use crate::{
        application::use_cases::tenant_user::update_tenant_user_custom_field_values::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                custom_field_definition::{CustomFieldDefinition, CustomFieldType},
                tenant::Tenant,
            },
            errors::DomainError,
            repositories::user_repository::UserRepository,
        },
        testing::{
            in_memory_repositories::{
                in_memory_custom_field_definition_repository::InMemoryCustomFieldDefinitionRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            user_factory::generate_user,
        },
    };

    fn make_tenant() -> Tenant {
        Tenant::new(
            "Test Tenant".to_string(),
            None,
            "test_secret".to_string(),
            true,
            true,
            true,
            Uuid::now_v7(),
        )
    }

    fn setup(
        tenant: &Tenant,
    ) -> (
        Arc<InMemoryUserRepository>,
        Arc<InMemoryCustomFieldDefinitionRepository>,
        UseCase,
    ) {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let fields = Arc::new(InMemoryCustomFieldDefinitionRepository::new());

        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(user_repo.clone(), tenant_repo.clone())
                .with_custom_field_definition_repository(fields.clone()),
        );

        let use_case = UseCase::new(uow_factory);

        (user_repo, fields, use_case)
    }

    #[tokio::test]
    async fn sets_custom_field_values_without_touching_name_fields() {
        let tenant = make_tenant();
        let (user_repo, fields, use_case) = setup(&tenant);

        let field = CustomFieldDefinition::new(
            tenant.id(),
            "nickname".to_string(),
            en("Nickname"),
            None,
            CustomFieldType::Text,
            None,
            false,
            0,
        );
        fields.memory.data.lock().unwrap().insert(field.id(), field);

        let user = generate_user(tenant.id());
        let original_first_name = user.profile().unwrap().first_name().to_string();
        let original_last_name = user.profile().unwrap().last_name().to_string();
        user_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    user_id: user.id(),
                    custom_field_values: Some(json!({"nickname": "gabs"})),
                },
            )
            .await
            .unwrap();

        assert_eq!(response.custom_field_values.unwrap()["nickname"], "gabs");

        // The whole point of this use case: name fields must be untouched.
        let saved = user_repo.find_by_id(user.id()).await.unwrap().unwrap();
        let saved_profile = saved.profile().unwrap();
        assert_eq!(saved_profile.first_name(), original_first_name);
        assert_eq!(saved_profile.last_name(), original_last_name);
        assert_eq!(
            saved_profile.custom_field_values().unwrap()["nickname"],
            "gabs"
        );
    }

    #[tokio::test]
    async fn fails_when_user_not_found() {
        let tenant = make_tenant();
        let (_, _, use_case) = setup(&tenant);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    user_id: Uuid::now_v7(),
                    custom_field_values: None,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_tenant_not_found() {
        let tenant = make_tenant();
        let (_, _, use_case) = setup(&tenant);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: Uuid::now_v7(),
                    user_id: Uuid::now_v7(),
                    custom_field_values: None,
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn rejects_unknown_custom_field_key() {
        let tenant = make_tenant();
        let (user_repo, _fields, use_case) = setup(&tenant);

        let user = generate_user(tenant.id());
        user_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: tenant.id(),
                    user_id: user.id(),
                    custom_field_values: Some(json!({"unknown_field": "x"})),
                },
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Validation(_)));
    }
}
