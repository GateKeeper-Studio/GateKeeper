use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Command {
    #[validate(length(min = 1, message = "Name cannot be empty"))]
    pub name: String,

    pub description: Option<String>,

    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}
