use chrono::{DateTime, Utc};
use uuid::Uuid;

// Which external IdP a configuration is for. Persisted as a lowercase
// discriminator string in external_identity_providers.provider, and surfaced on
// the wire so the login SPA can pick the right logo ("github" / "google").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalProviderKind {
    Google,
    Github,
}

impl ExternalProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExternalProviderKind::Google => "google",
            ExternalProviderKind::Github => "github",
        }
    }

    // Strict parse of the wire/admin-supplied discriminator. Returns None for an
    // unknown provider so the use case can reject it rather than silently coercing.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "google" => Some(ExternalProviderKind::Google),
            "github" => Some(ExternalProviderKind::Github),
            _ => None,
        }
    }

    // DB reconstruction. Stored values are always one of the known discriminators
    // (validated on write), so the fallback is unreachable in practice.
    pub fn from_db(value: &str) -> Self {
        ExternalProviderKind::parse(value).unwrap_or(ExternalProviderKind::Google)
    }
}

// A single external identity provider configured for one OAuth client (e.g. the
// "Sign in with Google" button on that client's login page).
//
// SECURITY: `provider_client_secret_encrypted` holds an AES-256-GCM ciphertext
// (see SecretCipher) and is decrypted only at the moment we call the provider's
// token endpoint. It is never exposed through any API projection.
#[derive(Debug, Clone)]
pub struct ExternalIdentityProvider {
    id: Uuid,
    // The OAuth client (internal id) this provider is enabled for.
    oauth_client_id: Uuid,
    // Denormalised tenant id (mirrors the client's tenant) for scoped lookups.
    tenant_id: Uuid,

    provider: ExternalProviderKind,
    // The OAuth client id/secret issued by the external provider (Google/GitHub).
    provider_client_id: String,
    provider_client_secret_encrypted: String,
    // The callback URL registered at the provider (must point back at our
    // /v1/auth/oauth-provider/{provider}/callback endpoint).
    redirect_uri: String,

    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl ExternalIdentityProvider {
    pub fn new(
        oauth_client_id: Uuid,
        tenant_id: Uuid,
        provider: ExternalProviderKind,
        provider_client_id: String,
        provider_client_secret_encrypted: String,
        redirect_uri: String,
        is_active: bool,
    ) -> Self {
        ExternalIdentityProvider {
            id: Uuid::now_v7(),
            oauth_client_id,
            tenant_id,
            provider,
            provider_client_id,
            provider_client_secret_encrypted,
            redirect_uri,
            is_active,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        oauth_client_id: Uuid,
        tenant_id: Uuid,
        provider: ExternalProviderKind,
        provider_client_id: String,
        provider_client_secret_encrypted: String,
        redirect_uri: String,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        ExternalIdentityProvider {
            id,
            oauth_client_id,
            tenant_id,
            provider,
            provider_client_id,
            provider_client_secret_encrypted,
            redirect_uri,
            is_active,
            created_at,
            updated_at,
        }
    }

    // Mutates the editable details (admin CRUD). The identity fields — id,
    // oauth_client_id, tenant_id, provider — are intentionally NOT changeable
    // here. `provider_client_secret_encrypted` is only updated when the admin
    // supplies a new secret (handled in the use case).
    pub fn update_details(
        &mut self,
        provider_client_id: String,
        provider_client_secret_encrypted: String,
        redirect_uri: String,
        is_active: bool,
    ) {
        self.provider_client_id = provider_client_id;
        self.provider_client_secret_encrypted = provider_client_secret_encrypted;
        self.redirect_uri = redirect_uri;
        self.is_active = is_active;
        self.updated_at = Some(Utc::now());
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn oauth_client_id(&self) -> Uuid {
        self.oauth_client_id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn provider(&self) -> ExternalProviderKind {
        self.provider
    }
    pub fn provider_client_id(&self) -> &str {
        &self.provider_client_id
    }
    pub fn provider_client_secret_encrypted(&self) -> &str {
        &self.provider_client_secret_encrypted
    }
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}
