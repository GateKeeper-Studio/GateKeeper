use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserProfile {
    user_id: Uuid,

    display_name: Option<String>,
    first_name: String,
    last_name: String,

    // Tenant-defined custom field answers for this user, keyed by field_key.
    // Stored as a validated JSON blob. None = nothing collected yet. Every value
    // inside is validated in the application layer (against the tenant's active
    // CustomFieldDefinitions) before it ever reaches this entity.
    custom_field_values: Option<Value>,

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
            custom_field_values: None,
            updated_at: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        user_id: Uuid,
        display_name: Option<String>,
        first_name: String,
        last_name: String,
        updated_at: Option<DateTime<Utc>>,
        custom_field_values: Option<Value>,
    ) -> Self {
        UserProfile {
            user_id,
            display_name,
            first_name,
            last_name,
            custom_field_values,
            updated_at,
        }
    }

    // Set or clear this user's custom field answers. A real mutation — bumps
    // `updated_at` — used when a user/admin actually submits new values.
    pub fn set_custom_field_values(&mut self, custom_field_values: Option<Value>) {
        self.custom_field_values = custom_field_values;
        self.updated_at = Some(Utc::now());
    }

    // Rehydrate the stored custom field values on load WITHOUT touching
    // `updated_at` (unlike `set_custom_field_values`, which is a real mutation).
    // Mirrors Tenant::hydrate_appearance.
    pub fn hydrate_custom_field_values(&mut self, custom_field_values: Option<Value>) {
        self.custom_field_values = custom_field_values;
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
    pub fn custom_field_values(&self) -> Option<&Value> {
        self.custom_field_values.as_ref()
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}
