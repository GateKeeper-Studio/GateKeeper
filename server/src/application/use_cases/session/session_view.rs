use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

// One row of the session list. Minimal by construction: which app, what it may do,
// and three times. No IP, no user agent (neither is stored for refresh tokens), and
// no token id or hash — `id` is the family id, which only ends the session.
#[derive(Debug, Clone, Serialize)]
pub struct SessionView {
    // The refresh-token family id; pass it to DELETE …/sessions/{id}.
    pub id: Uuid,
    // The app's PUBLIC client_id.
    pub client_id: String,
    pub client_name: Option<String>,
    pub scopes: Vec<String>,
    pub signed_in_at: DateTime<Utc>,
    // The last refresh. Only as precise as the access-token lifetime.
    pub last_active_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
