use thiserror::Error;

// The one error vocabulary of the inner layers. The message carries the detail
// (which entity, which field); the variant carries only what the HTTP layer needs
// to choose a status. It used to be 46 variants — mostly `XNotFound` /
// `XAlreadyExists` pairs that all mapped to the same two statuses.
#[derive(Debug, Error)]
pub enum DomainError {
    // ── Generic outcomes ─────────────────────────────────────────────────────
    /// The addressed resource does not exist (or is not visible to the caller).
    #[error("Not found: {0}")]
    NotFound(String),
    /// The write conflicts with existing state (duplicate name, already enrolled).
    #[error("Conflict: {0}")]
    Conflict(String),
    /// The input is invalid for a reason the caller can fix.
    #[error("Validation failed: {0}")]
    Validation(String),

    // ── Authentication ───────────────────────────────────────────────────────
    #[error("User is inactive: {0}")]
    UserInactive(String),
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    /// A destructive self-service action needs fresh proof (the current password).
    #[error("Step-up authentication required: {0}")]
    StepUpRequired(String),
    #[error("Email not confirmed: {0}")]
    EmailNotConfirmed(String),
    #[error("MFA verification failed: {0}")]
    MfaError(String),

    // ── OAuth 2.1 / OIDC protocol errors (RFC 6749 §5.2 codes) ──────────────────
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

    // ── Faults ───────────────────────────────────────────────────────────────
    /// An external system (SMTP server, social identity provider) failed or
    /// refused. The message is a fixed, caller-safe description; the raw upstream
    /// error is logged where it happens, never returned.
    #[error("Upstream provider error: {0}")]
    Upstream(String),
    #[error("Password hashing failed: {0}")]
    PasswordHashingError(String),
    #[error("Infrastructure error: {0}")]
    InfrastructureError(String),
}
