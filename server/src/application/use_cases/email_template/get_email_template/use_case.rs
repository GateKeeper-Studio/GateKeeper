use std::sync::Arc;

use crate::application::use_cases::email_template::get_email_template::{
    query::Query, response::Response,
};
use crate::domain::{
    errors::DomainError, repositories::email_template_repository::EmailTemplateRepository,
};

// Read-only: returns the tenant's template or the built-in default. Takes the
// repository arc directly per the read-use-case convention.
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
        let stored = self
            .email_template_repository
            .find_by_tenant_and_type(query.tenant_id, query.template_type)
            .await?;

        Ok(match stored {
            Some(template) => Response::customized(&template),
            None => Response::default_for(query.template_type),
        })
    }
}
