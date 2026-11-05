use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::entities::email_template::{EmailTemplate, EmailTemplateType};

/// Shared view returned by the get/update/reset email-template use cases.
///
/// `body_mjml` is the MJML source. `is_customized` is false when the tenant has
/// not overridden the template and the built-in default is being served (in which
/// case `updated_at` is `None`). `variables` is the situation's allow-list so the
/// editor can offer placeholder chips.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub template_type: String,
    pub subject: String,
    pub body_mjml: String,
    pub variables: Vec<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub is_customized: bool,
}

impl Response {
    /// A tenant-customized template.
    pub fn customized(template: &EmailTemplate) -> Self {
        Response {
            template_type: template.template_type().as_str().to_string(),
            subject: template.subject().to_string(),
            body_mjml: template.body_mjml().to_string(),
            variables: variables_of(template.template_type()),
            updated_at: Some(template.updated_at()),
            is_customized: true,
        }
    }

    /// The built-in default for a situation (no tenant override stored).
    pub fn default_for(template_type: EmailTemplateType) -> Self {
        Response {
            template_type: template_type.as_str().to_string(),
            subject: template_type.default_subject().to_string(),
            body_mjml: template_type.default_body_mjml().to_string(),
            variables: variables_of(template_type),
            updated_at: None,
            is_customized: false,
        }
    }
}

fn variables_of(template_type: EmailTemplateType) -> Vec<String> {
    template_type
        .variables()
        .iter()
        .map(|v| (*v).to_string())
        .collect()
}
