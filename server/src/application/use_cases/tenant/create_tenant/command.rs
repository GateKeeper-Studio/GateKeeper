use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    // The subscription this tenant joins (required).
    pub subscription_id: Uuid,

    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    pub description: Option<String>,

    #[validate(length(min = 1, message = "Password hash secret cannot be empty"))]
    pub password_hash_secret: String,

    pub can_self_register: bool,
    pub can_self_forgot_password: bool,
}
