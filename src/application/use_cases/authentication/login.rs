use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::{errors::DomainError};

pub struct LoginUseCase {
}

// WHY Validate derive: the `validator` crate gives you declarative validation
// rules as annotations. This keeps validation logic co-located with the struct
// instead of scattered across handlers.
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    pub tenant_id: Uuid,

    #[validate(length(min = 1, message = "Code challenge cannot be empty"))]
    pub code_challenge: String,

    #[validate(length(min = 1, message = "Code challenge method cannot be empty"))]
    pub code_challenge_method: String,

    #[validate(url(message = "Invalid redirect URI format"))]
    pub redirect_uri: String,

    #[validate(length(min = 1, message = "Response type cannot be empty"))]
    pub response_type: String,

    #[validate(length(min = 1, message = "Scope cannot be empty"))]
    pub scope: Option<String>,

    #[validate(length(min = 1, message = "State cannot be empty"))]
    pub state: String,

    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 6, message = "Password must be at least 6 characters long"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
  pub user_id: Uuid,
  pub message: String,
  pub mfa_id: Option<Uuid>,
  pub mfa_type: Option<String>,
  pub session_code: Option<String>,
  pub change_password_code: Option<String>,
  pub web_authn_options: Option<String>,
}

impl LoginUseCase {
  pub fn new() -> Self {
    LoginUseCase {  }
  }

  pub async fn execute(&self, request: LoginRequest) -> Result<LoginResponse, DomainError> {
    // Never log the full request — it contains the plaintext password.
    tracing::info!(tenant_id = %request.tenant_id, email = %request.email, "login attempt");

    todo!();
  }
}