use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::group::update_group_by_id::{command::Command, response::Response},
    },
    domain::errors::DomainError,
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

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?;
        if tenant.is_none() {
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        let mut group = uow
            .group_repository()
            .find_by_id(command.group_id)
            .await?
            .filter(|g| g.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::GroupNotFound(format!("Group with id {} not found", command.group_id))
            })?;

        let name = command.name.trim().to_string();

        // Re-check uniqueness only when the name actually changed.
        if name != group.name() {
            let name_taken = uow
                .group_repository()
                .is_group_with_name_exists(command.tenant_id, &name)
                .await?;
            if name_taken {
                return Err(DomainError::GroupAlreadyExists(format!(
                    "Group with name {name} already exists"
                )));
            }
        }

        let description = command
            .description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());

        group.update(name, description);

        uow.group_repository().update(&group).await?;
        let member_count = uow.group_repository().count_members(group.id()).await?;
        uow.commit().await?;

        Ok(Response {
            group_id: group.id(),
            name: group.name().to_string(),
            description: group.description().map(|s| s.to_string()),
            member_count,
            created_at: group.created_at().to_rfc3339(),
            updated_at: group.updated_at().map(|t| t.to_rfc3339()),
        })
    }
}
