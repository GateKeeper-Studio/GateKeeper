use uuid::Uuid;

use crate::domain::entities::legal_document::LegalDocumentType;

// Built in the handler from the path params; the reset request has no body.
pub struct Command {
    pub tenant_id: Uuid,
    pub document_type: LegalDocumentType,
}
