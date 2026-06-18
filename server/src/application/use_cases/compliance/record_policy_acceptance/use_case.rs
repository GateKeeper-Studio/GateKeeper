use std::sync::Arc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::compliance::record_policy_acceptance::{command::Command, response::Response},
    },
    domain::{entities::policy_acceptance::PolicyAcceptance, errors::DomainError},
};

// Write use case: records a (user, document_type, version) acceptance. Idempotent on
// the unique key — re-accepting the same version succeeds without duplicating. The
// `policy.accepted` audit event is emitted at the handler layer (it has the actor +
// request context).
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

        let acceptance = PolicyAcceptance::new(
            command.tenant_id,
            command.user_id,
            command.document_type,
            command.document_version,
            command.ip_address,
        );

        uow.policy_acceptance_repository()
            .record(&acceptance)
            .await?;
        uow.commit().await?;

        Ok(Response {
            document_type: acceptance.document_type().to_string(),
            document_version: acceptance.document_version().to_string(),
            accepted_at: acceptance.accepted_at().to_rfc3339(),
        })
    }
}
