use std::sync::Arc;

use uuid::Uuid;

use crate::{application::ports::unit_of_work::UnitOfWorkFactory, domain::errors::DomainError};

// Write use case: an admin removes a security block by id. Returns
// SecurityBlockNotFound (→ 404) when the id doesn't exist for the tenant. The
// `security.ip.unblocked` audit event is emitted at the handler layer.
pub struct Command {
    pub tenant_id: Uuid,
    pub block_id: Uuid,
}

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let removed = uow
            .security_block_repository()
            .delete(command.tenant_id, command.block_id)
            .await?;
        if !removed {
            return Err(DomainError::SecurityBlockNotFound(format!(
                "block {} not found",
                command.block_id
            )));
        }
        uow.commit().await?;
        Ok(())
    }
}
