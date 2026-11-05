use std::collections::HashSet;
use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::unit_of_work::UnitOfWorkFactory;
use crate::application::shared::actor::Actor;
use crate::application::use_cases::tenant_custom_field::reorder_tenant_custom_fields::command::Command;
use crate::domain::entities::custom_field_definition::CustomFieldDefinition;
use crate::domain::errors::DomainError;

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;
        let repo = uow.custom_field_definition_repository();

        let mut existing = repo.find_by_tenant_id(command.tenant_id).await?;

        let existing_ids: HashSet<Uuid> = existing.iter().map(CustomFieldDefinition::id).collect();
        let submitted_ids: HashSet<Uuid> = command.field_ids.iter().copied().collect();

        if submitted_ids.len() != command.field_ids.len() {
            return Err(DomainError::Validation(
                "field_ids contains duplicate ids".to_string(),
            ));
        }
        if existing_ids != submitted_ids {
            return Err(DomainError::Validation(
                "field_ids must be exactly the tenant's current set of custom field ids"
                    .to_string(),
            ));
        }

        for (index, field_id) in command.field_ids.iter().enumerate() {
            let definition = existing
                .iter_mut()
                .find(|def| def.id() == *field_id)
                .expect("field_id presence already verified by set-equality check above");

            let label = definition.label().clone();
            let description = definition.description().cloned();
            let options = definition.options().map(|o| o.to_vec());
            let is_required = definition.is_required();
            let is_active = definition.is_active();

            definition.update(
                label,
                description,
                options,
                is_required,
                index as i32,
                is_active,
            );

            repo.update(definition).await?;
        }

        uow.commit().await?;

        Ok(())
    }
}
