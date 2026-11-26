use std::sync::Arc;

use crate::{
    application::use_cases::admin_account::record_console_tour::command::Command,
    domain::{errors::DomainError, repositories::admin_user_repository::AdminUserRepository},
};

// Records that the signed-in admin finished or closed the console's guided tour, so
// it does not come back on first access in another browser. Only ever raises the
// stored version (see `AdminUserRepository::record_console_tour`).
pub struct UseCase {
    admin_repository: Arc<dyn AdminUserRepository>,
}

impl UseCase {
    pub fn new(admin_repository: Arc<dyn AdminUserRepository>) -> Self {
        Self { admin_repository }
    }

    pub async fn execute(&self, command: Command) -> Result<(), DomainError> {
        if !(1..=1000).contains(&command.version) {
            return Err(DomainError::Validation(
                "version must be 1-1000".to_string(),
            ));
        }
        self.admin_repository
            .record_console_tour(command.admin_id, command.version)
            .await
    }
}
