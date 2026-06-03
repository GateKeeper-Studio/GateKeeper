use crate::{
  application::use_cases::oauth::discovery::{query::Query, response::Response},
  domain::errors::DomainError,
};

pub struct UseCase {
  issuer: String,
}

impl UseCase {
  pub fn new(issuer: String) -> Self {
    Self { issuer }
  }

  pub async fn execute(&self, _query: Query) -> Result<Response, DomainError> {
    let base = self.issuer.trim_end_matches('/').to_string();

    Ok(Response {
      authorization_endpoint: format!("{base}/oauth/authorize"),
      token_endpoint: format!("{base}/oauth/token"),
      userinfo_endpoint: format!("{base}/oauth/userinfo"),
      jwks_uri: format!("{base}/.well-known/jwks.json"),
      issuer: base,
      response_types_supported: vec!["code".to_string()],
      grant_types_supported: vec![
        "authorization_code".to_string(),
        "refresh_token".to_string(),
      ],
      code_challenge_methods_supported: vec!["S256".to_string()],
      scopes_supported: vec![
        "openid".to_string(),
        "profile".to_string(),
        "email".to_string(),
        "offline_access".to_string(),
      ],
      token_endpoint_auth_methods_supported: vec![
        "client_secret_basic".to_string(),
        "client_secret_post".to_string(),
        "none".to_string(),
      ],
      id_token_signing_alg_values_supported: vec!["RS256".to_string(), "ES256".to_string()],
      subject_types_supported: vec!["public".to_string()],
    })
  }
}
