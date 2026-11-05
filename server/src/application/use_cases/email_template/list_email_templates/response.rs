use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::entities::email_template::EmailTemplateType;

/// One row in the dashboard's email-templates list. Built from the registry so a
/// new situation appears automatically once its enum variant is added.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub template_type: String,
    pub label: String,
    pub description: String,
    pub variables: Vec<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub is_customized: bool,
}

impl TemplateSummary {
    pub fn new(
        template_type: EmailTemplateType,
        updated_at: Option<DateTime<Utc>>,
        is_customized: bool,
    ) -> Self {
        TemplateSummary {
            template_type: template_type.as_str().to_string(),
            label: template_type.label().to_string(),
            description: template_type.description().to_string(),
            variables: template_type
                .variables()
                .iter()
                .map(|v| (*v).to_string())
                .collect(),
            updated_at,
            is_customized,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub items: Vec<TemplateSummary>,
}
