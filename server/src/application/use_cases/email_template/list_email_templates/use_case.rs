use std::sync::Arc;

use crate::application::use_cases::email_template::list_email_templates::{
    query::Query,
    response::{Response, TemplateSummary},
};
use crate::domain::{
    entities::email_template::EmailTemplateType, errors::DomainError,
    repositories::email_template_repository::EmailTemplateRepository,
};

// Read-only: returns every known situation (from the registry) annotated with
// whether the tenant has customized it. Drives the dashboard list.
pub struct UseCase {
    email_template_repository: Arc<dyn EmailTemplateRepository>,
}

impl UseCase {
    pub fn new(email_template_repository: Arc<dyn EmailTemplateRepository>) -> Self {
        Self {
            email_template_repository,
        }
    }

    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        let mut items = Vec::with_capacity(EmailTemplateType::ALL.len());

        for &template_type in EmailTemplateType::ALL {
            let stored = self
                .email_template_repository
                .find_by_tenant_and_type(query.tenant_id, template_type)
                .await?;

            let summary = match stored {
                Some(template) => {
                    TemplateSummary::new(template_type, Some(template.updated_at()), true)
                }
                None => TemplateSummary::new(template_type, None, false),
            };
            items.push(summary);
        }

        Ok(Response { items })
    }
}
