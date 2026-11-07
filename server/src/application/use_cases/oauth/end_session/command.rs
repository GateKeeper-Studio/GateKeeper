// The RP-Initiated Logout request (OIDC RP-Initiated Logout 1.0 §2), plus the
// request context the handler adds for the audit log.
#[derive(Debug, Clone, Default)]
pub struct Command {
    // An ID token this OP issued to the RP. Required here: with no OP session
    // cookie, it is the only thing that says who is signing out, and from which
    // session. May be expired.
    pub id_token_hint: Option<String>,
    // Optional; when present it must be the hint's audience.
    pub client_id: Option<String>,
    // Where to send the browser afterwards. Must exactly match one of the client's
    // registered post_logout_redirect_uris.
    pub post_logout_redirect_uri: Option<String>,
    // Echoed back on the redirect, for the RP's CSRF check.
    pub state: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
