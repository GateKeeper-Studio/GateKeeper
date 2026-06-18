use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::entities::legal_document::{LegalDocument, LegalDocumentType};

/// Shared view returned by every legal-document read/write use case.
///
/// `content` is markdown. `is_customized` is false when the tenant has not
/// overridden the document and the built-in default is being served (in which
/// case `updated_at` is `None`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub document_type: String,
    pub content: String,
    pub updated_at: Option<DateTime<Utc>>,
    pub is_customized: bool,
}

impl Response {
    /// A tenant-customized document.
    pub fn customized(document: &LegalDocument) -> Self {
        Response {
            document_type: document.document_type().as_str().to_string(),
            content: document.markdown_content().to_string(),
            updated_at: Some(document.updated_at()),
            is_customized: true,
        }
    }

    /// The built-in default for a document type (no tenant override stored).
    pub fn default_for(document_type: LegalDocumentType) -> Self {
        Response {
            document_type: document_type.as_str().to_string(),
            content: document_type.default_markdown(),
            updated_at: None,
            is_customized: false,
        }
    }
}
