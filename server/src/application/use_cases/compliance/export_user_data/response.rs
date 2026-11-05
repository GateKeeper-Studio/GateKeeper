use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

// The portable data bundle (GDPR Art. 15/20, LGPD Art. 18 II/V). Structured,
// machine-readable JSON of everything tied to the data subject — EXCLUDING
// credentials, token hashes, and signing-key material, which are never personal
// data of the subject and must not leak.
#[derive(Debug, Serialize)]
pub struct Response {
    pub generated_at: String,
    pub user: ExportedUser,
    pub consents: Vec<ExportedConsent>,
    pub roles: Vec<String>,
    pub groups: Vec<String>,
    // Placeholder until WI-04 lands; always present so the schema is stable.
    pub policy_acceptances: Vec<ExportedAcceptance>,
}

#[derive(Debug, Serialize)]
pub struct ExportedUser {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    // Tenant-defined data collected from this user (see tenant_custom_fields) —
    // must be included so the export is a complete "right to access" bundle.
    pub custom_field_values: Option<Value>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportedConsent {
    pub client_id: Uuid,
    pub scopes: Vec<String>,
    pub revoked: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportedAcceptance {
    pub document_type: String,
    pub document_version: String,
    pub accepted_at: String,
}
