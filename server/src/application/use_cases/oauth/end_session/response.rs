// What the logout endpoint answers with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    // The validated post-logout redirect (without `state`, which the handler appends),
    // or None when the RP asked for none.
    pub post_logout_redirect_uri: Option<String>,
    pub state: Option<String>,
    // How many of the user's sessions this request ended (0 when the session named by
    // the hint had already ended).
    pub sessions_ended: usize,
}
