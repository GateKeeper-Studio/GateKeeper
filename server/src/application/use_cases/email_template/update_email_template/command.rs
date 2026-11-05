use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::domain::entities::email_template::EmailTemplateType;

// `tenant_id` and `template_type` come from the path and are injected by the
// handler; only `subject` and `body_mjml` are read from the request body.
#[derive(Debug, Deserialize, Validate)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    #[serde(skip_deserializing, default)]
    pub template_type: EmailTemplateType,

    #[validate(length(
        min = 1,
        max = 300,
        message = "Subject must be between 1 and 300 characters"
    ))]
    pub subject: String,

    #[validate(length(
        min = 1,
        max = 100000,
        message = "Body must be between 1 and 100000 characters"
    ))]
    pub body_mjml: String,
}
