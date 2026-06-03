use chrono::{DateTime, Utc};
use uuid::Uuid;

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
  tenant_id: Uuid,                       // multi-tenant: a client belongs to a tenant

  client_id: String,                     // public identifier (opaque, random)
  client_secret_hash: Option<String>,    // Argon2 hash; None for public clients
  client_type: ClientType,

  name: String,
  // SECURITY: redirect URIs are matched EXACTLY (scheme+host+port+path+no wildcard).
  // This is the #1 defense against open-redirect / code-theft.
  redirect_uris: Vec<String>,
  post_logout_redirect_uris: Vec<String>,

  allowed_scopes: Vec<String>,           // the superset this client may ever request
  allowed_grant_types: Vec<String>,      // e.g. ["authorization_code", "refresh_token"]

  require_pkce: bool,                    // keep this TRUE for everyone
  // If true, every authorization needs a fresh consent screen; if false, consent
  // can be remembered (see ConsentGrant). First-party apps often set false.
  require_consent: bool,

  is_active: bool,
  created_at: DateTime<Utc>,
  updated_at: Option<DateTime<Utc>>,
}

impl OAuthClient {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    tenant_id: Uuid,
    client_id: String,
    client_secret_hash: Option<String>,
    client_type: ClientType,
    name: String,
    redirect_uris: Vec<String>,
    post_logout_redirect_uris: Vec<String>,
    allowed_scopes: Vec<String>,
    allowed_grant_types: Vec<String>,
    require_pkce: bool,
    require_consent: bool,
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
      is_active: true,
      created_at: Utc::now(),
      updated_at: None,
    }
  }

  #[allow(clippy::too_many_arguments)]
  pub fn reconstruct(
    id: Uuid,
    tenant_id: Uuid,
    client_id: String,
    client_secret_hash: Option<String>,
    client_type: ClientType,
    name: String,
    redirect_uris: Vec<String>,
    post_logout_redirect_uris: Vec<String>,
    allowed_scopes: Vec<String>,
    allowed_grant_types: Vec<String>,
    require_pkce: bool,
    require_consent: bool,
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
    is_active: bool,
  ) {
    self.name = name;
    self.redirect_uris = redirect_uris;
    self.post_logout_redirect_uris = post_logout_redirect_uris;
    self.allowed_scopes = allowed_scopes;
    self.allowed_grant_types = allowed_grant_types;
    self.require_pkce = require_pkce;
    self.require_consent = require_consent;
    self.is_active = is_active;
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
    requested.iter()
      .filter(|s| self.allowed_scopes.iter().any(|allowed| allowed == *s))
      .cloned()
      .collect()
  }

  pub fn is_confidential(&self) -> bool {
    matches!(self.client_type, ClientType::Confidential)
  }

  pub fn id(&self) -> Uuid { self.id }
  pub fn tenant_id(&self) -> Uuid { self.tenant_id }
  pub fn client_id(&self) -> &str { &self.client_id }
  pub fn client_secret_hash(&self) -> Option<&str> { self.client_secret_hash.as_deref() }
  pub fn client_type(&self) -> &ClientType { &self.client_type }
  pub fn name(&self) -> &str { &self.name }
  pub fn redirect_uris(&self) -> &[String] { &self.redirect_uris }
  pub fn post_logout_redirect_uris(&self) -> &[String] { &self.post_logout_redirect_uris }
  pub fn allowed_scopes(&self) -> &[String] { &self.allowed_scopes }
  pub fn allowed_grant_types(&self) -> &[String] { &self.allowed_grant_types }
  pub fn require_pkce(&self) -> bool { self.require_pkce }
  pub fn require_consent(&self) -> bool { self.require_consent }
  pub fn is_active(&self) -> bool { self.is_active }
  pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
  pub fn updated_at(&self) -> Option<DateTime<Utc>> { self.updated_at }
}