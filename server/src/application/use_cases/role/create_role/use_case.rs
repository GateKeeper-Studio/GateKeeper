use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::role::create_role::{command::Command, response::Response},
    },
    domain::{
        entities::role::{Role, is_reserved_role_name},
        errors::DomainError,
    },
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

        let name = command.name.trim().to_string();

        // A role name is privilege-bearing: it is embedded in the access token and
        // the admin router authorizes on it. Letting a tenant create `admin` would
        // let it grant its own users the admin surface.
        if is_reserved_role_name(&name) {
            return Err(DomainError::InvalidRequest(format!(
                "Role name '{name}' is reserved and cannot be used"
            )));
        }

        let name_taken = uow
            .role_repository()
            .is_role_with_name_exists(command.tenant_id, &name)
            .await?;
        if name_taken {
            return Err(DomainError::RoleAlreadyExists(format!(
                "Role with name {name} already exists"
            )));
        }

        let description = command
            .description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());

        let role = Role::new(command.tenant_id, name, description);

        uow.role_repository().add(&role).await?;
        uow.commit().await?;

        Ok(Response {
            role_id: role.id(),
            name: role.name().to_string(),
            description: role.description().map(|s| s.to_string()),
            user_count: 0,
            created_at: role.created_at().to_rfc3339(),
            updated_at: role.updated_at().map(|t| t.to_rfc3339()),
        })
    }
}
