use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::application::use_cases::email_template::{
    get_email_template::response::Response, reset_email_template::command::Command,
};
use crate::domain::repositories::tenant_repository::TenantRepository;
use crate::domain::{
    errors::DomainError, repositories::email_template_repository::EmailTemplateRepository,
};

// Removes a tenant's override so the built-in default is served again, and returns
// that default so the editor can refresh immediately.
pub struct UseCase {
    tenant_repository: Arc<dyn TenantRepository>,
    email_template_repository: Arc<dyn EmailTemplateRepository>,
}

impl UseCase {
    pub fn new(
        email_template_repository: Arc<dyn EmailTemplateRepository>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            email_template_repository,
            tenant_repository,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        actor
            .authorize_tenant_admin_for(self.tenant_repository.as_ref(), command.tenant_id)
            .await?;
        self.email_template_repository
            .delete(command.tenant_id, command.template_type)
            .await?;

        Ok(Response::default_for(command.template_type))
    }
}
