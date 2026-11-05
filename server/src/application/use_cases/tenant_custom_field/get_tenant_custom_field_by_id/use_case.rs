use std::sync::Arc;

use crate::{
    application::use_cases::tenant_custom_field::{
        custom_field_view::CustomFieldView,
        get_tenant_custom_field_by_id::{query::Query, response::Response},
    },
    domain::{
        errors::DomainError,
        repositories::custom_field_definition_repository::CustomFieldDefinitionRepository,
    },
};

pub struct UseCase {
    custom_field_definition_repository: Arc<dyn CustomFieldDefinitionRepository>,
}

impl UseCase {
    pub fn new(
        custom_field_definition_repository: Arc<dyn CustomFieldDefinitionRepository>,
    ) -> Self {
        Self {
            custom_field_definition_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let definition = self
            .custom_field_definition_repository
            .find_by_id(query.tenant_id, query.field_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("custom field {} not found", query.field_id))
            })?;

        Ok(CustomFieldView::from_entity(&definition))
    }
}
