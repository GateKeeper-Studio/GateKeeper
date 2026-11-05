use uuid::Uuid;

use crate::domain::entities::email_template::EmailTemplateType;

// Built in the handler from the path params; not deserialized from a request body.
pub struct Query {
    pub tenant_id: Uuid,
    pub template_type: EmailTemplateType,
}
