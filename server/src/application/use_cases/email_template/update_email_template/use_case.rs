use std::sync::Arc;

use crate::application::ports::email_renderer::EmailRenderer;
use crate::application::shared::actor::Actor;
use crate::application::use_cases::email_template::{
    get_email_template::response::Response, update_email_template::command::Command, validation,
};
use crate::domain::repositories::tenant_repository::TenantRepository;
use crate::domain::{
    entities::email_template::EmailTemplate, errors::DomainError,
    repositories::email_template_repository::EmailTemplateRepository,
};

// Stores (or replaces) a tenant's template. A single upsert is atomic on its own,
// so this takes the repository directly rather than a Unit of Work. The tenant FK
// on `email_templates` rejects writes for a non-existent tenant.
//
// Before persisting, the submitted MJML is validated (allow-listed placeholders,
// no raw passthrough) and test-compiled with the situation's sample context, so a
// template that cannot render never reaches the send path.
pub struct UseCase {
    tenant_repository: Arc<dyn TenantRepository>,
    email_template_repository: Arc<dyn EmailTemplateRepository>,
    renderer: Arc<dyn EmailRenderer>,
}

impl UseCase {
    pub fn new(
        email_template_repository: Arc<dyn EmailTemplateRepository>,
        renderer: Arc<dyn EmailRenderer>,
        tenant_repository: Arc<dyn TenantRepository>,
    ) -> Self {
        Self {
            email_template_repository,
            renderer,
            tenant_repository,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        actor
            .authorize_tenant_admin_for(self.tenant_repository.as_ref(), command.tenant_id)
            .await?;
        validation::validate(command.template_type, &command.subject, &command.body_mjml)?;

        // Test-compile with sample values; surfaces malformed MJML as a 400 here
        // rather than at send time.
        self.renderer
            .render(
                &command.subject,
                &command.body_mjml,
                &command.template_type.sample_context(),
            )
            .map_err(|e| DomainError::Validation(format!("template failed to render: {e}")))?;

        let template = EmailTemplate::new(
            command.tenant_id,
            command.template_type,
            command.subject,
            command.body_mjml,
        );

        self.email_template_repository.upsert(&template).await?;

        Ok(Response::customized(&template))
    }
}
