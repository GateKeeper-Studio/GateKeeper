use chrono::{DateTime, Utc};
use uuid::Uuid;

// Organization is the top-level owner of admins and tenants.
#[derive(Debug, Clone)]
pub struct Organization {
    id: Uuid,
    name: String,
    description: Option<String>,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Organization {
    pub fn new(name: String, description: Option<String>, is_active: bool) -> Self {
        Organization {
            id: Uuid::now_v7(),
            name,
            description,
            is_active,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn update(&mut self, name: String, description: Option<String>, is_active: bool) {
        self.name = name;
        self.description = description;
        self.is_active = is_active;
        self.updated_at = Some(Utc::now());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        name: String,
        description: Option<String>,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        Organization {
            id,
            name,
            description,
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
