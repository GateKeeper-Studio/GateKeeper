use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::group::create_group::{command::Command, response::Response},
    },
    domain::{entities::group::Group, errors::DomainError},
};

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

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?;
        if tenant.is_none() {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        let name = command.name.trim().to_string();

        let name_taken = uow
            .group_repository()
            .is_group_with_name_exists(command.tenant_id, &name)
            .await?;
        if name_taken {
            return Err(DomainError::Conflict(format!(
                "Group with name {name} already exists"
            )));
        }

        // Normalize an empty/whitespace-only description to None.
        let description = command
            .description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());

        let group = Group::new(command.tenant_id, name, description);

        uow.group_repository().add(&group).await?;
        uow.commit().await?;

        Ok(Response {
            group_id: group.id(),
            name: group.name().to_string(),
            description: group.description().map(|s| s.to_string()),
            member_count: 0,
            created_at: group.created_at().to_rfc3339(),
            updated_at: group.updated_at().map(|t| t.to_rfc3339()),
        })
    }
}
