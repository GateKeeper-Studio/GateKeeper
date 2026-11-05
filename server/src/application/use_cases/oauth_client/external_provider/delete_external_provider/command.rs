use uuid::Uuid;

// Removes a configured external provider. All ids come from the URL path; there
// is no request body.
pub struct Command {
    pub tenant_id: Uuid,
    pub oauth_client_id: Uuid,
    pub id: Uuid,
}
