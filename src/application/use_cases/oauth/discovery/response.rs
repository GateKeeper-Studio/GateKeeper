use serde::Serialize;

// OIDC Discovery metadata (OpenID Connect Discovery 1.0). Advertises only the
// secure-by-default surface: response_type=code, S256 PKCE, asymmetric signing.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
  pub issuer: String,
  pub authorization_endpoint: String,
  pub token_endpoint: String,
  pub userinfo_endpoint: String,
  pub jwks_uri: String,
  pub response_types_supported: Vec<String>,
  pub grant_types_supported: Vec<String>,
  pub code_challenge_methods_supported: Vec<String>,
  pub scopes_supported: Vec<String>,
  pub token_endpoint_auth_methods_supported: Vec<String>,
  pub id_token_signing_alg_values_supported: Vec<String>,
  pub subject_types_supported: Vec<String>,
}
