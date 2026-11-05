use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::tenant_custom_field::{
            create_tenant_custom_field::{command::Command, response::Response},
            custom_field_view::CustomFieldView,
            options_validation::{validate_localized_options, validate_localized_text},
        },
    },
    domain::{
        entities::custom_field_definition::{CustomFieldDefinition, CustomFieldType},
        errors::DomainError,
    },
};

const MAX_LABEL_LEN: usize = 150;
const MAX_DESCRIPTION_LEN: usize = 500;

// Deliberately generous but finite: keeps the admin-facing form and the
// per-signup validation pass bounded, and rules out using this as an
// unbounded per-tenant storage bucket.
const MAX_FIELDS_PER_TENANT: u32 = 40;

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::TenantNotFound(format!(
                    "Tenant with id {} not found",
                    command.tenant_id
                ))
            })?;
        let default_language = tenant.default_language();

        let field_key = command.field_key.trim().to_string();
        if !is_valid_field_key(&field_key) {
            return Err(DomainError::InvalidCustomFieldValue(
                "field_key must start with a lowercase letter and contain only lowercase letters, digits, and underscores".to_string(),
            ));
        }

        let field_type = CustomFieldType::parse(&command.field_type).ok_or_else(|| {
            DomainError::InvalidRequest(format!("unknown field_type: {}", command.field_type))
        })?;

        let repo = uow.custom_field_definition_repository();

        if repo
            .find_by_tenant_id_and_key(command.tenant_id, &field_key)
            .await?
            .is_some()
        {
            return Err(DomainError::CustomFieldKeyAlreadyExists(format!(
                "custom field '{field_key}' already exists for this tenant"
            )));
        }

        if repo.count_by_tenant_id(command.tenant_id).await? >= MAX_FIELDS_PER_TENANT {
            return Err(DomainError::CustomFieldLimitReached(format!(
                "tenant already has the maximum of {MAX_FIELDS_PER_TENANT} custom fields"
            )));
        }

        let label = validate_localized_text(
            "label",
            command.label,
            MAX_LABEL_LEN,
            Some(default_language),
        )?;
        let description = command
            .description
            .map(|d| validate_localized_text("description", d, MAX_DESCRIPTION_LEN, None))
            .transpose()?
            .filter(|d| !d.is_empty());
        let options = validate_localized_options(field_type, command.options, default_language)?;

        let definition = CustomFieldDefinition::new(
            command.tenant_id,
            field_key,
            label,
            description,
            field_type,
            options,
            command.is_required,
            command.display_order,
        );

        repo.add(&definition).await?;
        uow.commit().await?;

        Ok(CustomFieldView::from_entity(&definition))
    }
}

fn is_valid_field_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
