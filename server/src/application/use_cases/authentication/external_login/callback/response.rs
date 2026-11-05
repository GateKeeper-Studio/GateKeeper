// The pieces the handler needs to build the 302 back to the client app:
// `redirect_uri?code=<authorization_code>&state=<state>`. The raw authorization
// code travels in the URL exactly once.
#[derive(Debug)]
pub struct Response {
    pub redirect_uri: String,
    pub authorization_code: String,
    pub state: String,
}
