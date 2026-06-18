use chrono::{DateTime, Utc};
use uuid::Uuid;

// Group represents a tenant-scoped collection that users can be joined to.
#[derive(Debug, Clone)]
pub struct Group {
    id: Uuid,
    tenant_id: Uuid,
    name: String,
    description: Option<String>,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Group {
    pub fn new(tenant_id: Uuid, name: String, description: Option<String>) -> Self {
        Group {
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
        Group {
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
