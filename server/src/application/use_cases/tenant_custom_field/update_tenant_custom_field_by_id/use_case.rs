use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::tenant_custom_field::{
            custom_field_view::CustomFieldView,
            options_validation::{validate_localized_options, validate_localized_text},
            update_tenant_custom_field_by_id::{command::Command, response::Response},
        },
    },
    domain::errors::DomainError,
};

const MAX_LABEL_LEN: usize = 150;
const MAX_DESCRIPTION_LEN: usize = 500;

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        // Needed for the tenant's default_language — the required locale for
        // this field's label (and, if select, every option). Mirrors the
        // existence-check create_tenant_custom_field already does.
        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Tenant with id {} not found", command.tenant_id))
            })?;
        let default_language = tenant.default_language();

        let repo = uow.custom_field_definition_repository();

        let mut definition = repo
            .find_by_id(command.tenant_id, command.field_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("custom field {} not found", command.field_id))
            })?;

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

        // field_type is immutable, but options still need re-validating against
        // whatever type was fixed at creation (e.g. a `select` field's option list
        // can still be edited; a non-select field's options stay None).
        let options =
            validate_localized_options(definition.field_type(), command.options, default_language)?;

        definition.update(
            label,
            description,
            options,
            command.is_required,
            command.display_order,
            command.is_active,
        );

        repo.update(&definition).await?;
        uow.commit().await?;

        Ok(CustomFieldView::from_entity(&definition))
    }
}
