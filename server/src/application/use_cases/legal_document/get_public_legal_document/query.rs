use crate::domain::entities::legal_document::LegalDocumentType;

// Built in the handler from the path params. The public surface only knows the
// client_id, never the tenant UUID.
pub struct Query {
    pub client_id: String,
    pub document_type: LegalDocumentType,
}
