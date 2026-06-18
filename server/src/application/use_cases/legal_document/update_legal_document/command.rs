use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::domain::entities::legal_document::LegalDocumentType;

// `tenant_id` and `document_type` come from the path and are injected by the
// handler; only `markdown_content` is read from the request body.
#[derive(Debug, Deserialize, Validate)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,

    #[serde(skip_deserializing, default)]
    pub document_type: LegalDocumentType,

    #[validate(length(
        min = 1,
        max = 200000,
        message = "Content must be between 1 and 200000 characters"
    ))]
    pub markdown_content: String,
}
