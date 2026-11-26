use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub admin_id: Uuid,
    // A code from the authenticator app, proving it holds the seed.
    pub code: String,
}
