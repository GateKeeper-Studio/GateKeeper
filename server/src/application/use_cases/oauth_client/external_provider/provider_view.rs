use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::external_identity_provider::ExternalIdentityProvider;

// Admin-facing projection of a configured external identity provider. NEVER
// includes the client secret (not even its ciphertext) — secrets are write-only
// over the API. camelCase to match the dashboard front-end.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalProviderView {
    pub id: Uuid,
    pub oauth_client_id: Uuid,
    pub provider: String,
    pub provider_client_id: String,
    pub redirect_uri: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl ExternalProviderView {
    pub fn from_entity(provider: &ExternalIdentityProvider) -> Self {
        ExternalProviderView {
            id: provider.id(),
            oauth_client_id: provider.oauth_client_id(),
            provider: provider.provider().as_str().to_string(),
            provider_client_id: provider.provider_client_id().to_string(),
            redirect_uri: provider.redirect_uri().to_string(),
            is_active: provider.is_active(),
            created_at: provider.created_at().to_rfc3339(),
            updated_at: provider.updated_at().map(|t| t.to_rfc3339()),
        }
    }
}
