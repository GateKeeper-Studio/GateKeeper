use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::email_provider::EmailProvider;

// Public projection of an SMTP email provider. NEVER includes the password (not
// even its ciphertext) — secrets are write-only over the API. camelCase to match
// the dashboard front-end.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub from_email: String,
    pub from_name: Option<String>,
    pub encryption: String,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl ProviderView {
    pub fn from_entity(provider: &EmailProvider) -> Self {
        ProviderView {
            id: provider.id(),
            name: provider.name().to_string(),
            host: provider.host().to_string(),
            port: provider.port(),
            username: provider.username().to_string(),
            from_email: provider.from_email().to_string(),
            from_name: provider.from_name().map(str::to_string),
            encryption: provider.encryption().as_str().to_string(),
            is_default: provider.is_default(),
            is_active: provider.is_active(),
            created_at: provider.created_at().to_rfc3339(),
            updated_at: provider.updated_at().map(|t| t.to_rfc3339()),
        }
    }
}
