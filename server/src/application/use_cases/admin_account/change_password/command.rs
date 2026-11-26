use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

// The length rules live in `admin_password_policy`, applied by the use case.
#[derive(Debug, Deserialize, Validate)]
pub struct Command {
    // admin_id is injected from the verified token, not the request body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub admin_id: Uuid,

    // The console session making the request, injected by the handler: it stays
    // signed in while every other session of the admin ends.
    #[serde(skip_deserializing, default)]
    pub keep_session_id: Option<Uuid>,

    // Re-authentication: the current password must be presented to change it, so a
    // hijacked-but-unlocked session can't silently set a new one.
    pub current_password: String,

    pub new_password: String,
}
