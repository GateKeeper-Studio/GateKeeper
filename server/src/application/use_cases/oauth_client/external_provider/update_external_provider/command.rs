use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// Updates a configured external provider. The provider kind is immutable (it is
// the client's other Google/GitHub identity); only the credentials, redirect and
// active flag change. `client_secret` is optional: omit (or send empty) to keep
// the stored secret, supply a value to rotate it.
#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub oauth_client_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub id: Uuid,

    #[validate(length(min = 1, message = "client id cannot be empty"))]
    pub client_id: String,

    pub client_secret: Option<String>,

    #[validate(url(message = "a valid redirect URI is required"))]
    pub redirect_uri: String,

    pub is_active: bool,
}
