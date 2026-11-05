use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Role names that grant access to the admin surface.
///
/// A user's roles are resolved from tenant RBAC and embedded in their access
/// token; the auth layer authorizes the admin router by testing membership in
/// this list (`presentation::middleware::auth::ADMIN_ROLES` re-exports it). That
/// makes the names privilege-bearing, so tenants must not be able to mint them:
/// a customer creating a role called `admin` — the most common role name there
/// is — would otherwise hand that role's members the admin surface.
///
/// Lives in the domain because both the authorization check (presentation) and
/// the creation rule (application) need it, and application may not depend on
/// presentation.
pub const ADMIN_ROLE_NAMES: [&str; 2] = ["admin", "tenant_admin"];

/// Whether `name` collides with a privilege-bearing role name. Compared
/// case-insensitively on the trimmed value, because that is how a near-miss
/// would be attempted (`Admin`, `ADMIN`, `" admin "`).
pub fn is_reserved_role_name(name: &str) -> bool {
    let name = name.trim();
    ADMIN_ROLE_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(name))
}

// Role represents a tenant-scoped named permission set that users can be assigned to.
#[derive(Debug, Clone)]
pub struct Role {
    id: Uuid,
    tenant_id: Uuid,
    name: String,
    description: Option<String>,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Role {
    pub fn new(tenant_id: Uuid, name: String, description: Option<String>) -> Self {
        Role {
            id: Uuid::now_v7(),
            tenant_id,
            name,
            description,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn update(&mut self, name: String, description: Option<String>) {
        self.name = name;
        self.description = description;
        self.updated_at = Some(Utc::now());
    }

    pub fn reconstruct(
        id: Uuid,
        tenant_id: Uuid,
        name: String,
        description: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        Role {
            id,
            tenant_id,
            name,
            description,
            created_at,
            updated_at,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
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
}
