use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

// Tenant is an isolated workspace owned by an organization.
#[derive(Debug, Clone)]
pub struct Tenant {
    id: Uuid,
    name: String,
    description: Option<String>,

    is_active: bool,

    can_self_register: bool, // Whether users can self-register under this tenant
    can_self_forgot_password: bool, // Whether users can use forgot password flow
    password_hash_secret: String,

    // The top-level organization that owns this tenant.
    organization_id: Uuid,

    // Per-tenant dashboard appearance (main color + component style tokens),
    // stored as a validated JSON blob. None = use the default theme. Every value
    // inside is validated in the application layer before it reaches this entity.
    appearance: Option<Value>,

    // Fallback/required language for this tenant's custom sign-up fields (see
    // CustomFieldDefinition) — a validated locale code, e.g. "en". Defaults to
    // "en" on creation; editable afterward via `update()`.
    default_language: String,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Tenant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        description: Option<String>,
        password_hash_secret: String,
        is_active: bool,
        can_self_register: bool,
        can_self_forgot_password: bool,
        organization_id: Uuid,
    ) -> Self {
        Tenant {
            id: Uuid::now_v7(),
            name,
            description,
            is_active,
            can_self_register,
            can_self_forgot_password,
            password_hash_secret,
            organization_id,
            appearance: None,
            default_language: "en".to_string(),
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        name: String,
        description: Option<String>,
        is_active: bool,
        can_self_register: bool,
        can_self_forgot_password: bool,
        default_language: String,
    ) {
        self.name = name;
        self.description = description;
        self.is_active = is_active;
        self.can_self_register = can_self_register;
        self.can_self_forgot_password = can_self_forgot_password;
        self.default_language = default_language;
        self.updated_at = Some(Utc::now());
    }

    // Set or clear the per-organization dashboard appearance. Passing None reverts
    // the tenant to the default theme. Bumps `updated_at` like any other mutation.
    pub fn set_appearance(&mut self, appearance: Option<Value>) {
        self.appearance = appearance;
        self.updated_at = Some(Utc::now());
    }

    // Rehydrate the stored appearance on load WITHOUT touching `updated_at` (unlike
    // set_appearance, which is a real mutation). Kept out of `reconstruct` so the
    // many existing call sites don't change — mirrors OAuthClient::set_login_appearance.
    pub fn hydrate_appearance(&mut self, appearance: Option<Value>) {
        self.appearance = appearance;
    }

    // Rehydrate the stored default_language on load WITHOUT touching `updated_at`.
    // Kept out of `reconstruct` for the same reason as `hydrate_appearance` — so the
    // many existing call sites (mostly test fixtures) don't need a new argument.
    pub fn hydrate_default_language(&mut self, default_language: String) {
        self.default_language = default_language;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        name: String,
        description: Option<String>,
        password_hash_secret: String,
        is_active: bool,
        can_self_register: bool,
        can_self_forgot_password: bool,
        organization_id: Uuid,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        Tenant {
            id,
            name,
            description,
            is_active,
            can_self_register,
            can_self_forgot_password,
            password_hash_secret,
            organization_id,
            appearance: None,
            default_language: "en".to_string(),
            created_at,
            updated_at,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
    pub fn password_hash_secret(&self) -> &str {
        &self.password_hash_secret
    }
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    pub fn can_self_register(&self) -> bool {
        self.can_self_register
    }
    pub fn can_self_forgot_password(&self) -> bool {
        self.can_self_forgot_password
    }
    pub fn organization_id(&self) -> Uuid {
        self.organization_id
    }
    pub fn appearance(&self) -> Option<&Value> {
        self.appearance.as_ref()
    }
    pub fn default_language(&self) -> &str {
        &self.default_language
    }
}
