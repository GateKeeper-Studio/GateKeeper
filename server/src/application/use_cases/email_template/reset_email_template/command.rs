use uuid::Uuid;

use crate::domain::entities::email_template::EmailTemplateType;

// Built in the handler from the path params; the reset request has no body.
pub struct Command {
    pub tenant_id: Uuid,
    pub template_type: EmailTemplateType,
}
