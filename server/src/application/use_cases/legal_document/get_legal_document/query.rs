use uuid::Uuid;

use crate::domain::entities::legal_document::LegalDocumentType;

// Built in the handler from the path params; not deserialized from a request body.
pub struct Query {
    pub tenant_id: Uuid,
    pub document_type: LegalDocumentType,
}
