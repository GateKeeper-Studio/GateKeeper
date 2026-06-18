use chrono::{DateTime, Utc};
use uuid::Uuid;

// A subscription groups multiple tenants under one plan. Every subscription owns
// exactly one default ("master") tenant, created together with it.
#[derive(Debug, Clone)]
pub struct Subscription {
    id: Uuid,
    // The organization that owns this subscription (one subscription per org).
    organization_id: Uuid,
    plan_id: Uuid,
    is_active: bool,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Subscription {
    pub fn new(organization_id: Uuid, plan_id: Uuid, is_active: bool) -> Self {
        Subscription {
            id: Uuid::now_v7(),
            organization_id,
            plan_id,
            is_active,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    // The owning organization is fixed at creation; only plan/active change.
    pub fn update(&mut self, plan_id: Uuid, is_active: bool) {
        self.plan_id = plan_id;
        self.is_active = is_active;
        self.updated_at = Some(Utc::now());
    }

    pub fn reconstruct(
        id: Uuid,
        organization_id: Uuid,
        plan_id: Uuid,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        Subscription {
            id,
            organization_id,
            plan_id,
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
    pub fn plan_id(&self) -> Uuid {
        self.plan_id
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
