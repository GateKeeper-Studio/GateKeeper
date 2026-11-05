use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::group::add_group_members::{command::Command, response::Response},
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

        let group = uow
            .group_repository()
            .find_by_id(command.group_id)
            .await?
            .filter(|g| g.tenant_id() == command.tenant_id)
            .ok_or_else(|| {
                DomainError::NotFound(format!("Group with id {} not found", command.group_id))
            })?;

        // Validate every user belongs to this tenant before joining any, so a bad
        // id rolls back the whole batch.
        for user_id in &command.user_ids {
            let belongs = uow
                .user_repository()
                .find_by_id(*user_id)
                .await?
                .map(|u| u.tenant_id() == command.tenant_id)
                .unwrap_or(false);

            if !belongs {
                return Err(DomainError::NotFound(format!(
                    "User with id {user_id} not found"
                )));
            }
        }

        for user_id in &command.user_ids {
            // Idempotent: the repository upserts membership (ON CONFLICT DO NOTHING).
            uow.group_repository()
                .add_member(group.id(), *user_id)
                .await?;
        }

        let member_count = uow.group_repository().count_members(group.id()).await?;
        uow.commit().await?;

        Ok(Response {
            added: command.user_ids.len() as u32,
            member_count,
        })
    }
}
