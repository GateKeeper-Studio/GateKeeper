use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserProfile {
    user_id: Uuid,

    display_name: Option<String>,
    first_name: String,
    last_name: String,

    updated_at: Option<DateTime<Utc>>,
}

impl UserProfile {
    pub fn new(
        user_id: Uuid,
        display_name: Option<String>,
        first_name: String,
        last_name: String,
    ) -> Self {
        UserProfile {
            user_id,
            display_name,
            first_name,
            last_name,
            updated_at: None,
        }
    }

    pub fn reconstruct(
        user_id: Uuid,
        display_name: Option<String>,
        first_name: String,
        last_name: String,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        UserProfile {
            user_id,
            display_name,
            first_name,
            last_name,
            updated_at,
        }
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn display_name(&self) -> Option<&String> {
        self.display_name.as_ref()
    }
    pub fn first_name(&self) -> &str {
        &self.first_name
    }
    pub fn last_name(&self) -> &str {
        &self.last_name
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}
