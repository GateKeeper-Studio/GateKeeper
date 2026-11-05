use std::collections::HashMap;

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

// Which pre-built login-page template this client's hosted auth screens use. The
// value is a whitelisted key that selects one of OUR Svelte layout components —
// the admin never supplies markup, so there is no code-injection surface here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginLayout {
    Centered,
    SplitImage,
    Card,
    Minimal,
}

impl LoginLayout {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoginLayout::Centered => "centered",
            LoginLayout::SplitImage => "split_image",
            LoginLayout::Card => "card",
            LoginLayout::Minimal => "minimal",
        }
    }

    // Returns None for unknown keys so the application layer can reject them and a
    // corrupted/legacy DB value can fall back to a safe default on read.
    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "centered" => Some(LoginLayout::Centered),
            "split_image" => Some(LoginLayout::SplitImage),
            "card" => Some(LoginLayout::Card),
            "minimal" => Some(LoginLayout::Minimal),
            _ => None,
        }
    }
}

// Which side the hero panel sits on for the split-image layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroSide {
    Left,
    Right,
}

impl HeroSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            HeroSide::Left => "left",
            HeroSide::Right => "right",
        }
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "left" => Some(HeroSide::Left),
            "right" => Some(HeroSide::Right),
            _ => None,
        }
    }
}

// Font family for the login screens. A closed allowlist (never raw user input):
// the key maps to a bundled, safe font stack in the front-end. Keeping it an enum
// means an admin can't inject an arbitrary `font-family` value into the CSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginFont {
    Inter,
    System,
    Roboto,
    Georgia,
}

impl LoginFont {
    pub fn as_str(&self) -> &'static str {
        match self {
            LoginFont::Inter => "inter",
            LoginFont::System => "system",
            LoginFont::Roboto => "roboto",
            LoginFont::Georgia => "georgia",
        }
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "inter" => Some(LoginFont::Inter),
            "system" => Some(LoginFont::System),
            "roboto" => Some(LoginFont::Roboto),
            "georgia" => Some(LoginFont::Georgia),
            _ => None,
        }
    }
}

// Per-client login-page appearance: the chosen layout template plus a small set of
// validated branding tokens. Every field is either a whitelisted enum, a validated
// http(s) URL, a hex color, a constrained length/unit value, or plain text — so the
// hosted login UI can render it WITHOUT ever executing admin-supplied code. The
// invariants are enforced at the application boundary (see validation.rs), mirroring
// how tos_uri / policy_uri are validated before they reach this entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAppearance {
    pub layout: LoginLayout,
    pub hero_position: HeroSide,
    pub logo_uri: Option<String>,
    pub background_image_uri: Option<String>,
    // locale -> text. Not required to cover every allowed_language — a hosted
    // page falls back through the client's default_language, then whatever
    // translation exists (see application::shared::localization::resolve_localized).
    pub heading: Option<HashMap<String, String>>,
    pub subtext: Option<HashMap<String, String>>,
    pub primary_color: Option<String>,
    pub accent_color: Option<String>,
    pub border_radius: Option<String>,
    pub font_family: Option<LoginFont>,
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

    // OIDC `initiate_login_uri` (RFC 7591 / OIDC Dynamic Client Registration): the RP
    // endpoint that starts a fresh authorization request.
    //
    // The hosted login UI sends the browser here when the OAuth state/PKCE it was handed
    // has expired. It has to be the RP that restarts the flow, not us: the PKCE
    // code_verifier lives in the RP's own cookie and we have never seen it, so only the
    // RP can mint a replacement. Registering the URI here — rather than accepting one in
    // a query parameter — is what keeps "restart my sign-in" from being an open redirect.
    initiate_login_uri: Option<String>,

    // Optional per-client login-page template + branding. None → the default look.
    login_appearance: Option<LoginAppearance>,

    // When true, this client authenticates ONLY through its configured external
    // identity providers: every password-credential path (self-registration, password
    // sign-in, forgot/reset password, forced change) is refused for it, and its users
    // never pick a password at all. The social callback already provisions passwordless
    // accounts, so nothing else about the flow changes.
    //
    // The application layer refuses to turn this on unless the client has at least one
    // ACTIVE external provider — otherwise the hosted login page would offer no way in.
    external_login_only: bool,

    // Which of the supported locales (domain::locale::SUPPORTED_LOCALES) this
    // client's hosted login/sign-up page offers to end users, and which one it
    // uses absent an explicit user choice. Always non-empty; default_language is
    // always a member (enforced in the application layer before either reaches
    // here — see validate_client_languages).
    allowed_languages: Vec<String>,
    default_language: String,

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
        allowed_languages: Vec<String>,
        default_language: String,
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
            initiate_login_uri: None,
            login_appearance: None,
            external_login_only: false,
            allowed_languages,
            default_language,
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
        initiate_login_uri: Option<String>,
        allowed_languages: Vec<String>,
        default_language: String,
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
            initiate_login_uri,
            // Rehydrated separately via set_login_appearance() / set_external_login_only()
            // so this constructor's signature stays stable across the many call sites that
            // build clients.
            login_appearance: None,
            external_login_only: false,
            allowed_languages,
            default_language,
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
        initiate_login_uri: Option<String>,
        login_appearance: Option<LoginAppearance>,
        external_login_only: bool,
        allowed_languages: Vec<String>,
        default_language: String,
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
        self.initiate_login_uri = initiate_login_uri;
        self.login_appearance = login_appearance;
        self.external_login_only = external_login_only;
        self.allowed_languages = allowed_languages;
        self.default_language = default_language;
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

    // Set/clear the RP endpoint that restarts an expired authorization request (admin
    // CRUD on create, where `new` defaults it to None).
    pub fn set_initiate_login_uri(&mut self, initiate_login_uri: Option<String>) {
        self.initiate_login_uri = initiate_login_uri;
    }

    // Set/clear the per-client login-page appearance. Used on create and when
    // rehydrating from the database (reconstruct leaves it None). Does not touch
    // updated_at — it is not, by itself, an admin edit event. Pass None for the
    // default look.
    pub fn set_login_appearance(&mut self, login_appearance: Option<LoginAppearance>) {
        self.login_appearance = login_appearance;
    }

    // Set the external-provider-only policy. Used on create and when rehydrating from
    // the database (reconstruct leaves it false), mirroring set_login_appearance —
    // admin edits go through update_details, which stamps updated_at.
    pub fn set_external_login_only(&mut self, external_login_only: bool) {
        self.external_login_only = external_login_only;
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
    pub fn initiate_login_uri(&self) -> Option<&str> {
        self.initiate_login_uri.as_deref()
    }
    pub fn login_appearance(&self) -> Option<&LoginAppearance> {
        self.login_appearance.as_ref()
    }
    pub fn external_login_only(&self) -> bool {
        self.external_login_only
    }
    // The question every password-credential use case actually asks before it spends
    // an Argon2 verification or mints a reset token.
    pub fn allows_password_login(&self) -> bool {
        !self.external_login_only
    }
    pub fn allowed_languages(&self) -> &[String] {
        &self.allowed_languages
    }
    pub fn default_language(&self) -> &str {
        &self.default_language
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
