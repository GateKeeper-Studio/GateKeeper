use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::compliance::purge_expired_data::response::Response,
    },
    domain::errors::DomainError,
};

// Retention enforcement (WI-03, storage limitation). Hard-deletes already-expired
// session/token/code rows in one transaction. Idempotent — re-running purges nothing
// new. The audit log is NEVER touched here (its retention is a separate, longer
// floor). `data.purged` is emitted at the handler layer with the run's counts.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(&self) -> Result<Response, DomainError> {
        let now = Utc::now();
        let uow = self.unit_of_work_factory.begin().await?;

        let refresh_tokens = uow.refresh_token_repository().delete_expired(now).await?;
        let auth_sessions = uow.auth_session_repository().delete_expired(now).await?;
        let authorization_codes = uow
            .authorization_code_repository()
            .delete_expired(now)
            .await?;

        uow.commit().await?;

        let total = refresh_tokens + auth_sessions + authorization_codes;
        Ok(Response {
            refresh_tokens,
            auth_sessions,
            authorization_codes,
            total,
        })
    }
}
