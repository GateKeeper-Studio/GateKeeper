
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
  #[error("Invalid email address: {0}")]
  InvalidEmail(String),
  #[error("Invalid first name: {0}")]
  InvalidFirstName(String),
  #[error("Invalid last name: {0}")]
  InvalidLastName(String),

  #[error("User not found: {0}")]
  UserNotFound(String),
  #[error("User already exists: {0}")]
  UserAlreadyExists(String),
  #[error("User is inactive: {0}")]
  UserInactive(String),

  #[error("Tenant not found: {0}")]
  TenantNotFound(String),
  #[error("Tenant already exists: {0}")]
  TenantAlreadyExists(String),

  #[error("Authentication failed: {0}")]
  AuthenticationError(String),

  #[error("Password hashing failed: {0}")]
  PasswordHashingError(String),

  #[error("Infrastructure error: {0}")]
  InfrastructureError(String),

  #[error("OAuth client not found: {0}")]
  OAuthClientNotFound(String),
  #[error("Invalid client: {0}")]
  InvalidClient(String),
  #[error("Invalid grant: {0}")]
  InvalidGrant(String),
  #[error("Invalid scope: {0}")]
  InvalidScope(String),
  #[error("Unauthorized client: {0}")]
  UnauthorizedClient(String),
  #[error("Invalid authorization request: {0}")]
  InvalidRequest(String),
  #[error("Access denied: {0}")]
  AccessDenied(String),
  
  #[error("Login required: {0}")]
  LoginRequired(String),
  #[error("Consent required: {0}")]
  ConsentRequired(String),
}