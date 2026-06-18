use chrono::{DateTime, Utc};
use uuid::Uuid;

// Tenant represents an organization or group that can have multiple users and resources.
#[derive(Debug, Clone)]
pub struct Tenant {
    id: Uuid,
    name: String,
    description: Option<String>,

    is_active: bool,

    can_self_register: bool, // Whether users can self-register under this tenant
    can_self_forgot_password: bool, // Whether users can use forgot password flow
    password_hash_secret: String,

    // The subscription this tenant belongs to. `is_default` marks the single
    // "master" tenant auto-created with the subscription.
    subscription_id: Uuid,
    is_default: bool,

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
        subscription_id: Uuid,
        is_default: bool,
    ) -> Self {
        Tenant {
            id: Uuid::now_v7(),
            name,
            description,
            is_active,
            can_self_register,
            can_self_forgot_password,
            password_hash_secret,
            subscription_id,
            is_default,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn update(
        &mut self,
        name: String,
        description: Option<String>,
        is_active: bool,
        can_self_register: bool,
        can_self_forgot_password: bool,
    ) {
        self.name = name;
        self.description = description;
        self.is_active = is_active;
        self.can_self_register = can_self_register;
        self.can_self_forgot_password = can_self_forgot_password;
        self.updated_at = Some(Utc::now());
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
        subscription_id: Uuid,
        is_default: bool,
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
            subscription_id,
            is_default,
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
    pub fn subscription_id(&self) -> Uuid {
        self.subscription_id
    }
    pub fn is_default(&self) -> bool {
        self.is_default
    }
}
