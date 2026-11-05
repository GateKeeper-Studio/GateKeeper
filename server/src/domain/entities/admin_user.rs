use chrono::{DateTime, Utc};
use uuid::Uuid;

// A platform administrator account. Global (not tenant-scoped): one admin operates
// across all tenants via the dashboard's admin surface. The password is hashed with
// Argon2id (a server-wide pepper); the algorithm string records which scheme so the
// hash stays self-describing if it is ever migrated.
#[derive(Debug, Clone)]
pub struct AdminUser {
    id: Uuid,
    // The organization this admin belongs to (an organization has many admins).
    organization_id: Uuid,
    email: String,
    // Human display name shown in the dashboard. May be empty for accounts bootstrapped
    // before the admin set one; the UI falls back to deriving a label from the email.
    name: String,
    password_hash: String,
    password_algorithm: String,
    is_active: bool,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl AdminUser {
    pub fn new(
        organization_id: Uuid,
        email: String,
        name: String,
        password_hash: String,
        password_algorithm: String,
    ) -> Self {
        AdminUser {
            id: Uuid::now_v7(),
            organization_id,
            email,
            name,
            password_hash,
            password_algorithm,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        organization_id: Uuid,
        email: String,
        name: String,
        password_hash: String,
        password_algorithm: String,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        AdminUser {
            id,
            organization_id,
            email,
            name,
            password_hash,
            password_algorithm,
            is_active,
            created_at,
            updated_at,
        }
    }

    // ── Self-service mutations (the account page) ────────────────────────────────
    // Each stamps `updated_at` so the change is observable. Email uniqueness is
    // enforced by the repository (the `admin_users` unique index), not here.

    pub fn rename(&mut self, name: String) {
        self.name = name;
        self.touch();
    }

    pub fn change_email(&mut self, email: String) {
        self.email = email;
        self.touch();
    }

    pub fn set_password(&mut self, password_hash: String, password_algorithm: String) {
        self.password_hash = password_hash;
        self.password_algorithm = password_algorithm;
        self.touch();
    }

    // Soft-deactivate (reversible). Login and token refresh both reject an inactive
    // admin, so flipping this flag ends access; the row is kept for audit history.
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = Some(Utc::now());
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn organization_id(&self) -> Uuid {
        self.organization_id
    }
    pub fn email(&self) -> &str {
        &self.email
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }
    pub fn password_algorithm(&self) -> &str {
        &self.password_algorithm
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
