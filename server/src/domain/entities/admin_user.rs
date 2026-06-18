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
        password_hash: String,
        password_algorithm: String,
    ) -> Self {
        AdminUser {
            id: Uuid::now_v7(),
            organization_id,
            email,
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
            password_hash,
            password_algorithm,
            is_active,
            created_at,
            updated_at,
        }
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
