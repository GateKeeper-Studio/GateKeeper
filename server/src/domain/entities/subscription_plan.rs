use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::entities::billing_interval::BillingInterval;

// A subscription plan is a CRUD-managed catalog entry: a named tier with pricing
// and per-subscription limits that subscriptions are bound to.
#[derive(Debug, Clone)]
pub struct SubscriptionPlan {
    id: Uuid,
    name: String,
    description: Option<String>,

    price_cents: i64,
    billing_interval: BillingInterval,
    max_tenants: i32,
    max_users: i32,

    is_active: bool,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl SubscriptionPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        description: Option<String>,
        price_cents: i64,
        billing_interval: BillingInterval,
        max_tenants: i32,
        max_users: i32,
        is_active: bool,
    ) -> Self {
        SubscriptionPlan {
            id: Uuid::now_v7(),
            name,
            description,
            price_cents,
            billing_interval,
            max_tenants,
            max_users,
            is_active,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        name: String,
        description: Option<String>,
        price_cents: i64,
        billing_interval: BillingInterval,
        max_tenants: i32,
        max_users: i32,
        is_active: bool,
    ) {
        self.name = name;
        self.description = description;
        self.price_cents = price_cents;
        self.billing_interval = billing_interval;
        self.max_tenants = max_tenants;
        self.max_users = max_users;
        self.is_active = is_active;
        self.updated_at = Some(Utc::now());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        name: String,
        description: Option<String>,
        price_cents: i64,
        billing_interval: BillingInterval,
        max_tenants: i32,
        max_users: i32,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        SubscriptionPlan {
            id,
            name,
            description,
            price_cents,
            billing_interval,
            max_tenants,
            max_users,
            is_active,
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
    pub fn price_cents(&self) -> i64 {
        self.price_cents
    }
    pub fn billing_interval(&self) -> BillingInterval {
        self.billing_interval
    }
    pub fn max_tenants(&self) -> i32 {
        self.max_tenants
    }
    pub fn max_users(&self) -> i32 {
        self.max_users
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
