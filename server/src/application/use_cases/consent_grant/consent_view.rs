use serde::Serialize;
use uuid::Uuid;

// One row on the user's "connected apps" screen: which client they authorized and
// the scopes it currently holds. The client name is enriched from the oauth_client
// record so the UI shows something human-readable, not a UUID.
#[derive(Debug, Clone, Serialize)]
pub struct ConsentView {
    pub client_id: Uuid, // internal oauth_clients.id (used to revoke)
    pub client_name: Option<String>,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}
