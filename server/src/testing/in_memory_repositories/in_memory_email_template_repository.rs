use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::email_template::{EmailTemplate, EmailTemplateType},
    errors::DomainError,
    repositories::email_template_repository::EmailTemplateRepository,
};

/// Simple in-memory store for unit tests. Writes apply immediately (these use
/// cases don't go through the Unit of Work), so no draft/commit machinery.
pub struct InMemoryEmailTemplateRepository {
    templates: Mutex<HashMap<(Uuid, EmailTemplateType), EmailTemplate>>,
}

impl InMemoryEmailTemplateRepository {
    pub fn new() -> Self {
        Self {
            templates: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryEmailTemplateRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailTemplateRepository for InMemoryEmailTemplateRepository {
    async fn find_by_tenant_and_type(
        &self,
        tenant_id: Uuid,
        template_type: EmailTemplateType,
    ) -> Result<Option<EmailTemplate>, DomainError> {
        let templates = self.templates.lock().unwrap();
        Ok(templates.get(&(tenant_id, template_type)).cloned())
    }

    async fn upsert(&self, template: &EmailTemplate) -> Result<(), DomainError> {
        let mut templates = self.templates.lock().unwrap();
        templates.insert(
            (template.tenant_id(), template.template_type()),
            template.clone(),
        );
        Ok(())
    }

    async fn delete(
        &self,
        tenant_id: Uuid,
        template_type: EmailTemplateType,
    ) -> Result<(), DomainError> {
        let mut templates = self.templates.lock().unwrap();
        templates.remove(&(tenant_id, template_type));
        Ok(())
    }
}
