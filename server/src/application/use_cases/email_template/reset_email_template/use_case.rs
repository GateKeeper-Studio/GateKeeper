use std::sync::Arc;

use crate::application::use_cases::email_template::{
    get_email_template::response::Response, reset_email_template::command::Command,
};
use crate::domain::{
    errors::DomainError, repositories::email_template_repository::EmailTemplateRepository,
};

// Removes a tenant's override so the built-in default is served again, and returns
// that default so the editor can refresh immediately.
pub struct UseCase {
    email_template_repository: Arc<dyn EmailTemplateRepository>,
}

impl UseCase {
    pub fn new(email_template_repository: Arc<dyn EmailTemplateRepository>) -> Self {
        Self {
            email_template_repository,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        self.email_template_repository
            .delete(command.tenant_id, command.template_type)
            .await?;

        Ok(Response::default_for(command.template_type))
    }
}
