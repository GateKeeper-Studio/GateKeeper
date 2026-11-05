use serde::Deserialize;
use validator::Validate;

use crate::domain::entities::email_template::EmailTemplateType;

// `template_type` comes from the path; `subject` and `body_mjml` are the *draft*
// the editor is currently showing (not necessarily saved), so the preview matches
// exactly what would be sent.
#[derive(Debug, Deserialize, Validate)]
pub struct Command {
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
