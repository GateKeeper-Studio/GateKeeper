use uuid::Uuid;

// Lists the external providers configured for one client. Both ids come from the
// URL path; there is no request body.
pub struct Query {
    pub tenant_id: Uuid,
    pub oauth_client_id: Uuid,
}
