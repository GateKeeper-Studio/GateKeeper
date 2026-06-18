use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::mfa_method::MfaMethod;

// Per-client MFA policy: whether a second factor is mandatory for this client's
// users, and which factors the client accepts. Kept as one value object so the
// policy travels together through constructors and repositories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MfaPolicy {
    pub required: bool,
    pub email_enabled: bool,
    pub totp_enabled: bool,
    pub webauthn_enabled: bool,
}

impl Default for MfaPolicy {
    // MFA off, but every method pre-enabled, so flipping `required` on
    // immediately offers all factors.
    fn default() -> Self {
        MfaPolicy {
            required: false,
            email_enabled: true,
            totp_enabled: true,
            webauthn_enabled: true,
        }
    }
}

impl MfaPolicy {
    pub fn allows(&self, method: &MfaMethod) -> bool {
        match method {
            MfaMethod::Email => self.email_enabled,
            MfaMethod::Totp => self.totp_enabled,
            MfaMethod::Webauthn => self.webauthn_enabled,
        }
    }
}

// Public clients (SPAs, mobile, native) CANNOT keep a secret → no client_secret,
// PKCE is their only protection. Confidential clients (server-side web apps) CAN
// keep a secret and must authenticate at the token endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientType {
    Public,
    Confidential,
}

impl ClientType {
    // Persisted as a lowercase discriminator string in oauth_clients.client_type.
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientType::Public => "public",
            ClientType::Confidential => "confidential",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "confidential" => ClientType::Confidential,
            _ => ClientType::Public,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OAuthClient {
    id: Uuid,
    tenant_id: Uuid, // multi-tenant: a client belongs to a tenant

    client_id: Uuid,                    // public identifier (UUID v7)
    client_secret_hash: Option<String>, // Argon2 hash; None for public clients
    client_type: ClientType,

    name: String,
    // SECURITY: redirect URIs are matched EXACTLY (scheme+host+port+path+no wildcard).
    // This is the #1 defense against open-redirect / code-theft.
    redirect_uris: Vec<String>,
    post_logout_redirect_uris: Vec<String>,

    allowed_scopes: Vec<String>, // the superset this client may ever request
    allowed_grant_types: Vec<String>, // e.g. ["authorization_code", "refresh_token"]

    require_pkce: bool, // keep this TRUE for everyone
    // If true, every authorization needs a fresh consent screen; if false, consent
    // can be remembered (see ConsentGrant). First-party apps often set false.
    require_consent: bool,

    // Which tenant SMTP provider sends this client's emails. None → fall back to
    // the tenant's default provider (and ultimately the dev log sender).
    email_provider_id: Option<Uuid>,

    // Optional per-client legal documents (OIDC tos_uri / policy_uri, RFC 7591).
    // When set, the hosted login UI links to the client application's own Terms
    // of Service / Privacy Policy; None falls back to the platform pages.
    tos_uri: Option<String>,
    policy_uri: Option<String>,

    mfa_policy: MfaPolicy,

    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl OAuthClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: Uuid,
        client_id: Uuid,
        client_secret_hash: Option<String>,
        client_type: ClientType,
        name: String,
        redirect_uris: Vec<String>,
        post_logout_redirect_uris: Vec<String>,
        allowed_scopes: Vec<String>,
        allowed_grant_types: Vec<String>,
        require_pkce: bool,
        require_consent: bool,
        mfa_policy: MfaPolicy,
    ) -> Self {
        OAuthClient {
            id: Uuid::now_v7(),
            tenant_id,
            client_id,
            client_secret_hash,
            client_type,
            name,
            redirect_uris,
            post_logout_redirect_uris,
            allowed_scopes,
            allowed_grant_types,
            require_pkce,
            require_consent,
            email_provider_id: None,
            tos_uri: None,
            policy_uri: None,
            mfa_policy,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        client_id: Uuid,
        client_secret_hash: Option<String>,
        client_type: ClientType,
        name: String,
        redirect_uris: Vec<String>,
        post_logout_redirect_uris: Vec<String>,
        allowed_scopes: Vec<String>,
        allowed_grant_types: Vec<String>,
        require_pkce: bool,
        require_consent: bool,
        email_provider_id: Option<Uuid>,
        tos_uri: Option<String>,
        policy_uri: Option<String>,
        mfa_policy: MfaPolicy,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        OAuthClient {
            id,
            tenant_id,
            client_id,
            client_secret_hash,
            client_type,
            name,
            redirect_uris,
            post_logout_redirect_uris,
            allowed_scopes,
            allowed_grant_types,
            require_pkce,
            require_consent,
            email_provider_id,
            tos_uri,
            policy_uri,
            mfa_policy,
            is_active,
            created_at,
            updated_at,
        }
    }

    // Mutates the editable details of a client (admin CRUD). The identity-bearing
    // fields — id, client_id, client_secret_hash, client_type — are intentionally
    // NOT changeable here: rotating a secret or re-issuing a client_id is a
    // separate, deliberate operation.
    #[allow(clippy::too_many_arguments)]
    pub fn update_details(
        &mut self,
        name: String,
        redirect_uris: Vec<String>,
        post_logout_redirect_uris: Vec<String>,
        allowed_scopes: Vec<String>,
        allowed_grant_types: Vec<String>,
        require_pkce: bool,
        require_consent: bool,
        email_provider_id: Option<Uuid>,
        tos_uri: Option<String>,
        policy_uri: Option<String>,
        mfa_policy: MfaPolicy,
        is_active: bool,
    ) {
        self.name = name;
        self.redirect_uris = redirect_uris;
        self.post_logout_redirect_uris = post_logout_redirect_uris;
        self.allowed_scopes = allowed_scopes;
        self.allowed_grant_types = allowed_grant_types;
        self.require_pkce = require_pkce;
        self.require_consent = require_consent;
        self.email_provider_id = email_provider_id;
        self.tos_uri = tos_uri;
        self.policy_uri = policy_uri;
        self.mfa_policy = mfa_policy;
        self.is_active = is_active;
        self.updated_at = Some(Utc::now());
    }

    // Bind/unbind the SMTP provider this client sends through (admin CRUD on create,
    // where `new` defaults it to None).
    pub fn set_email_provider_id(&mut self, email_provider_id: Option<Uuid>) {
        self.email_provider_id = email_provider_id;
    }

    // Set/clear the per-client legal document URLs (admin CRUD on create, where
    // `new` defaults them to None). Pass None to fall back to the platform pages.
    pub fn set_legal_uris(&mut self, tos_uri: Option<String>, policy_uri: Option<String>) {
        self.tos_uri = tos_uri;
        self.policy_uri = policy_uri;
    }

    // Rotate the client secret: replace the stored Argon2 hash with a freshly
    // generated one, invalidating the previous secret. The caller hashes the new
    // raw secret (with the tenant pepper) and is responsible for returning the raw
    // value to the operator exactly once — it is never recoverable afterwards.
    pub fn rotate_secret_hash(&mut self, client_secret_hash: String) {
        self.client_secret_hash = Some(client_secret_hash);
        self.updated_at = Some(Utc::now());
    }

    // Exact-match only. Never use `starts_with`, never allow wildcards.
    pub fn is_redirect_uri_allowed(&self, uri: &str) -> bool {
        self.redirect_uris.iter().any(|allowed| allowed == uri)
    }

    pub fn is_post_logout_redirect_allowed(&self, uri: &str) -> bool {
        self.post_logout_redirect_uris.iter().any(|u| u == uri)
    }

    pub fn allows_grant(&self, grant: &str) -> bool {
        self.allowed_grant_types.iter().any(|g| g == grant)
    }

    pub fn filter_scopes<'a>(&self, requested: &'a [String]) -> Vec<String> {
        requested
            .iter()
            .filter(|s| self.allowed_scopes.iter().any(|allowed| allowed == *s))
            .cloned()
            .collect()
    }

    pub fn is_confidential(&self) -> bool {
        matches!(self.client_type, ClientType::Confidential)
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn client_id(&self) -> Uuid {
        self.client_id
    }
    pub fn client_secret_hash(&self) -> Option<&str> {
        self.client_secret_hash.as_deref()
    }
    pub fn client_type(&self) -> &ClientType {
        &self.client_type
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn redirect_uris(&self) -> &[String] {
        &self.redirect_uris
    }
    pub fn post_logout_redirect_uris(&self) -> &[String] {
        &self.post_logout_redirect_uris
    }
    pub fn allowed_scopes(&self) -> &[String] {
        &self.allowed_scopes
    }
    pub fn allowed_grant_types(&self) -> &[String] {
        &self.allowed_grant_types
    }
    pub fn require_pkce(&self) -> bool {
        self.require_pkce
    }
    pub fn require_consent(&self) -> bool {
        self.require_consent
    }
    pub fn email_provider_id(&self) -> Option<Uuid> {
        self.email_provider_id
    }
    pub fn tos_uri(&self) -> Option<&str> {
        self.tos_uri.as_deref()
    }
    pub fn policy_uri(&self) -> Option<&str> {
        self.policy_uri.as_deref()
    }
    pub fn mfa_policy(&self) -> &MfaPolicy {
        &self.mfa_policy
    }
    pub fn mfa_required(&self) -> bool {
        self.mfa_policy.required
    }
    pub fn allows_mfa_method(&self, method: &MfaMethod) -> bool {
        self.mfa_policy.allows(method)
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
