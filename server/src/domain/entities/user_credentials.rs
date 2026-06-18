use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserCredentials {
    user_id: Uuid,
    password_hash: String,
    password_hash_algorithm: String,
    should_change_password: bool,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl UserCredentials {
    pub fn new(
        user_id: Uuid,
        password_hash: String,
        password_hash_algorithm: String,
        should_change_password: bool,
    ) -> Self {
        UserCredentials {
            user_id,
            password_hash,
            password_hash_algorithm,
            should_change_password,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }
    pub fn password_hash_algorithm(&self) -> &str {
        &self.password_hash_algorithm
    }
    pub fn should_change_password(&self) -> bool {
        self.should_change_password
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}
