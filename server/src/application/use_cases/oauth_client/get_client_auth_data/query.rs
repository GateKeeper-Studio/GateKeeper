#[derive(Debug, Clone)]
pub struct Query {
    pub client_id: String, // public, globally-unique OAuth client_id
    // The locale the hosted page is currently rendering in (from its `?locale=`
    // query param). None or an unrecognized value just falls back to the
    // client's own default_language — never an error, since this is a public,
    // unauthenticated endpoint.
    pub requested_locale: Option<String>,
}
