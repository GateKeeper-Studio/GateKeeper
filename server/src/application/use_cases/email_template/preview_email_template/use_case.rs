use std::sync::Arc;

use crate::application::ports::email_renderer::EmailRenderer;
use crate::application::use_cases::email_template::preview_email_template::{
    command::Command, response::Response,
};
use crate::domain::errors::DomainError;

// Renders the draft template with the situation's sample values through the exact
// same pipeline used at send time (MJML compile -> scrub -> inline). Read-only.
pub struct UseCase {
    renderer: Arc<dyn EmailRenderer>,
}

impl UseCase {
    pub fn new(renderer: Arc<dyn EmailRenderer>) -> Self {
        Self { renderer }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let rendered = self
            .renderer
            .render(
                &command.subject,
                &command.body_mjml,
                &command.template_type.sample_context(),
            )
            .map_err(|e| DomainError::Validation(format!("template failed to render: {e}")))?;

        Ok(Response {
            subject: rendered.subject,
            html: rendered.html,
        })
    }
}
