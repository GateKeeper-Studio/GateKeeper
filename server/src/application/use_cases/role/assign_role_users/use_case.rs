use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::role::assign_role_users::{command::Command, response::Response},
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

        let role = uow
            .role_repository()
            .find_by_id(command.role_id)
            .await?
            .filter(|r| r.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::RoleNotFound(format!("Role with id {} not found", command.role_id))
            })?;

        // Validate every user belongs to this tenant before assigning any.
        for user_id in &command.user_ids {
            let belongs = uow
                .user_repository()
                .find_by_id(*user_id)
                .await?
                .map(|u| u.tenant_id() == command.tenant_id)
                .unwrap_or(false);

            if !belongs {
                return Err(DomainError::UserNotFound(format!(
                    "User with id {user_id} not found"
                )));
            }
        }

        for user_id in &command.user_ids {
            // Idempotent: ON CONFLICT DO NOTHING in the repository.
            uow.role_repository()
                .assign_user(role.id(), *user_id)
                .await?;
        }

        let user_count = uow
            .role_repository()
            .count_assigned_users(role.id())
            .await?;
        uow.commit().await?;

        Ok(Response {
            assigned: command.user_ids.len() as u32,
            user_count,
        })
    }
}
