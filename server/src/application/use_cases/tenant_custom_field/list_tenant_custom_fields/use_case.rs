use std::sync::Arc;

use crate::{
    application::use_cases::tenant_custom_field::{
        custom_field_view::CustomFieldView,
        list_tenant_custom_fields::{query::Query, response::Response},
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
        // All fields (active + inactive) for the admin management view.
        let definitions = self
            .custom_field_definition_repository
            .find_by_tenant_id(query.tenant_id)
            .await?;

        Ok(Response {
            fields: definitions
                .iter()
                .map(CustomFieldView::from_entity)
                .collect(),
        })
    }
}
