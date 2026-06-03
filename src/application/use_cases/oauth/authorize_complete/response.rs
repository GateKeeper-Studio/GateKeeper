// The data the handler needs to 302 the browser back to the client:
//   {redirect_uri}?code={code}&state={state}
// `code` is the RAW authorization code — returned exactly once here; only its
// SHA-256 hash was persisted.
#[derive(Debug, Clone)]
pub struct Response {
  pub redirect_uri: String,
  pub code: String,
  pub state: String,
}
