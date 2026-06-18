use serde::Serialize;

use crate::domain::entities::policy_acceptance::PolicyAcceptance;

#[derive(Debug, Serialize)]
pub struct Response {
    pub document_type: String,
    pub document_version: String,
    pub accepted_at: String,
}

impl From<PolicyAcceptance> for Response {
    fn from(a: PolicyAcceptance) -> Self {
        Response {
            document_type: a.document_type().to_string(),
            document_version: a.document_version().to_string(),
            accepted_at: a.accepted_at().to_rfc3339(),
        }
    }
}
