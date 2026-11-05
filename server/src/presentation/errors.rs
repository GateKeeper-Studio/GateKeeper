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
    // A destructive self-service action needs the current password (step-up).
    StepUpRequired(String),
    // An external system (SMTP server, social identity provider) failed. The
    // message is a fixed description; the raw upstream error is only logged.
    BadGateway(String),
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
            ApiError::StepUpRequired(msg) => (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "step_up_required", "error_description": msg })),
            )
                .into_response(),
            ApiError::BadGateway(msg) => (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": "upstream_error", "error_description": msg })),
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
            DomainError::NotFound(message) => ApiError::NotFound(message),
            DomainError::Conflict(message) => ApiError::Conflict(message),
            DomainError::Validation(message)
            | DomainError::InvalidRequest(message)
            | DomainError::InvalidScope(message)
            | DomainError::InvalidGrant(message) => ApiError::BadRequest(message),
            DomainError::AuthenticationError(message)
            | DomainError::InvalidClient(message)
            | DomainError::UnauthorizedClient(message)
            | DomainError::MfaError(message) => ApiError::Unauthorized(message),
            DomainError::AccessDenied(message) => ApiError::Forbidden(message),
            DomainError::StepUpRequired(message) => ApiError::StepUpRequired(message),
            DomainError::LoginRequired(message) | DomainError::ConsentRequired(message) => {
                ApiError::Redirected(message)
            }
            DomainError::EmailNotConfirmed(message) => ApiError::EmailNotConfirmed(message),
            DomainError::Upstream(message) => ApiError::BadGateway(message),
            // The client only ever sees a generic 500, so record the real cause here;
            // otherwise the server log has nothing but "500 Internal Server Error".
            other @ (DomainError::UserInactive(_)
            | DomainError::PasswordHashingError(_)
            | DomainError::InfrastructureError(_)) => {
                tracing::error!(error = ?other, "unhandled domain error mapped to 500");
                ApiError::Internal
            }
        }
    }
}
