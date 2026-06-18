use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

// One usable (or pending) second factor. Email has no enrollment row, so it
// appears as a synthetic entry with `id: None`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MfaMethodView {
    pub id: Option<Uuid>,
    pub method: String,
    pub name: Option<String>,
    pub confirmed: bool,
    pub is_preferred: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    // The method the login flow challenges first ("email"|"totp"|"webauthn");
    // None means the login priority chain decides (webauthn > totp > email).
    pub preferred_method: Option<String>,
    pub methods: Vec<MfaMethodView>,
}
