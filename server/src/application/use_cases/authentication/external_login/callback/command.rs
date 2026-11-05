// The external provider's redirect back to us. All three values arrive in the
// callback URL: `provider_name` from the path, `code` and `state` from the query.
pub struct Command {
    pub provider_name: String,
    pub code: String,
    pub state: String,
    // Injected server-side from the request context (never client-supplied), so a
    // social sign-in is geo-enriched and risk-scored like an interactive login.
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
