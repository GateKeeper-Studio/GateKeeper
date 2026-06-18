use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::role::update_role_by_id::{command::Command, response::Response},
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

        let mut role = uow
            .role_repository()
            .find_by_id(command.role_id)
            .await?
            .filter(|r| r.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::RoleNotFound(format!("Role with id {} not found", command.role_id))
            })?;

        let name = command.name.trim().to_string();

        if name != role.name() {
            let name_taken = uow
                .role_repository()
                .is_role_with_name_exists(command.tenant_id, &name)
                .await?;
            if name_taken {
                return Err(DomainError::RoleAlreadyExists(format!(
                    "Role with name {name} already exists"
                )));
            }
        }

        let description = command
            .description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());

        role.update(name, description);

        uow.role_repository().update(&role).await?;
        let user_count = uow
            .role_repository()
            .count_assigned_users(role.id())
            .await?;
        uow.commit().await?;

        Ok(Response {
            role_id: role.id(),
            name: role.name().to_string(),
            description: role.description().map(|s| s.to_string()),
            user_count,
            created_at: role.created_at().to_rfc3339(),
            updated_at: role.updated_at().map(|t| t.to_rfc3339()),
        })
    }
}
