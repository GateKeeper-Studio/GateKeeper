use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use serde_json::json;
use validator::ValidationErrors;

use crate::domain::errors::DomainError;

/// Custom JSON extractor that converts deserialization failures into `ApiError`
/// instead of Axum's raw `JsonRejection`, so all error responses are structured.
pub struct AppJson<T>(pub T);

impl<T, S> axum::extract::FromRequest<S> for AppJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(AppJson(value)),
            Err(rejection) => Err(ApiError::from(rejection)),
        }
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        let msg = match &rejection {
            JsonRejection::JsonDataError(_) | JsonRejection::JsonSyntaxError(_) => {
                let raw = rejection.body_text();
                // Strip the " at line X column Y" location suffix for a cleaner message.
                match raw.find(" at line ") {
                    Some(pos) => raw[..pos].to_string(),
                    None => raw,
                }
            }
            JsonRejection::MissingJsonContentType(_) => {
                "missing Content-Type: application/json header".to_string()
            }
            _ => return ApiError::Internal,
        };

        ApiError::BadRequest(msg)
    }
}

#[derive(Debug)]
pub enum ApiError {
    Forbidden(String),
    Redirected(String),
    Unauthorized(String),
    NotFound(String),
    Conflict(String),
    BadRequest(String),
    // The user authenticated but has not yet confirmed their email. The login SPA
    // keys off the `title` to route them to the confirm-email screen, so this
    // carries a structured `{ title, message }` body rather than the bare `{ error }`.
    EmailNotConfirmed(String),
    Validation(ValidationErrors),
    Internal,
}

// IntoResponse converts ApiError into an actual HTTP response.
// Axum requires this to be implemented for your error type so handlers can
// return Result<T, ApiError>.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Forbidden(msg) => {
                (StatusCode::FORBIDDEN, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::Redirected(msg) => {
                (StatusCode::FOUND, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::Conflict(msg) => {
                (StatusCode::CONFLICT, Json(json!({ "error": msg }))).into_response()
            }
            ApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "errors": [{ "message": msg }] })),
            )
                .into_response(),
            ApiError::EmailNotConfirmed(msg) => (
                StatusCode::FORBIDDEN,
                Json(json!({ "title": "E-mail not confirmed", "message": msg })),
            )
                .into_response(),
            ApiError::Validation(errors) => {
                let field_errors: Vec<serde_json::Value> = errors
                    .field_errors()
                    .iter()
                    .flat_map(|(field, errs)| {
                        errs.iter().map(move |e| {
                            json!({
                              "field": field,
                              "message": e.message
                                .as_ref()
                                .map(|m| m.to_string())
                                .unwrap_or_else(|| e.code.to_string()),
                            })
                        })
                    })
                    .collect();
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(json!({ "errors": field_errors })),
                )
                    .into_response()
            }
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
                .into_response(),
        }
    }
}

// The From trait defines how DomainError converts to ApiError.
// WHY: the ? operator calls From::from() automatically. So inside a handler
// you can write `use_case.execute().await?` and Rust will automatically
// convert DomainError → ApiError.
impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::LoginRequired(message) => ApiError::Redirected(message),
            DomainError::AccessDenied(message) => ApiError::Forbidden(message),
            DomainError::UnauthorizedClient(message) => ApiError::Unauthorized(message),
            DomainError::InvalidClient(message) => ApiError::Unauthorized(message),
            DomainError::AuthenticationError(message) => ApiError::Unauthorized(message),
            DomainError::OAuthClientNotFound(message) => ApiError::NotFound(message),
            // OAuth protocol error codes that are client mistakes, not server faults.
            DomainError::InvalidRequest(message) => ApiError::BadRequest(message),
            DomainError::InvalidScope(message) => ApiError::BadRequest(message),
            DomainError::InvalidGrant(message) => ApiError::BadRequest(message),
            DomainError::UserNotFound(message) => ApiError::NotFound(message),
            DomainError::UserAlreadyExists(message) => ApiError::Conflict(message),
            DomainError::TenantNotFound(message) => ApiError::NotFound(message),
            DomainError::TenantAlreadyExists(message) => ApiError::Conflict(message),
            DomainError::CannotDeleteDefaultTenant(message) => ApiError::Conflict(message),
            DomainError::OrganizationNotFound(message) => ApiError::NotFound(message),
            DomainError::OrganizationAlreadyExists(message) => ApiError::Conflict(message),
            DomainError::OrganizationAlreadyHasSubscription(message) => ApiError::Conflict(message),
            DomainError::SubscriptionNotFound(message) => ApiError::NotFound(message),
            DomainError::SubscriptionPlanNotFound(message) => ApiError::NotFound(message),
            DomainError::SubscriptionPlanAlreadyExists(message) => ApiError::Conflict(message),
            DomainError::SubscriptionPlanInactive(message) => ApiError::Conflict(message),
            DomainError::SubscriptionPlanInUse(message) => ApiError::Conflict(message),
            DomainError::SubscriptionLimitReached(message) => ApiError::Conflict(message),
            DomainError::PlanDowngradeNotAllowed(message) => ApiError::Conflict(message),
            DomainError::GroupNotFound(message) => ApiError::NotFound(message),
            DomainError::GroupAlreadyExists(message) => ApiError::Conflict(message),
            DomainError::RoleNotFound(message) => ApiError::NotFound(message),
            DomainError::RoleAlreadyExists(message) => ApiError::Conflict(message),
            DomainError::InvalidEmail(message) => ApiError::BadRequest(message),
            DomainError::InvalidFirstName(message) => ApiError::BadRequest(message),
            DomainError::InvalidLastName(message) => ApiError::BadRequest(message),
            DomainError::ConsentRequired(message) => ApiError::Redirected(message),
            DomainError::EmailVerificationError(message) => ApiError::BadRequest(message),
            DomainError::EmailNotConfirmed(message) => ApiError::EmailNotConfirmed(message),
            DomainError::MfaError(message) => ApiError::Unauthorized(message),
            DomainError::MfaEnrollmentNotFound(message) => ApiError::NotFound(message),
            DomainError::MfaEnrollmentConflict(message) => ApiError::Conflict(message),
            DomainError::PasswordResetError(message) => ApiError::BadRequest(message),
            DomainError::PasswordChangeError(message) => ApiError::BadRequest(message),
            DomainError::EmailProviderNotFound(message) => ApiError::NotFound(message),
            DomainError::EmailProviderError(message) => ApiError::BadRequest(message),
            _ => ApiError::Internal,
        }
    }
}
