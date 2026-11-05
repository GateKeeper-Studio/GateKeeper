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
    #[error("Organization not found: {0}")]
    OrganizationNotFound(String),
    #[error("Organization already exists: {0}")]
    OrganizationAlreadyExists(String),

    #[error("Group not found: {0}")]
    GroupNotFound(String),
    #[error("Group already exists: {0}")]
    GroupAlreadyExists(String),

    #[error("Role not found: {0}")]
    RoleNotFound(String),
    #[error("Role already exists: {0}")]
    RoleAlreadyExists(String),

    #[error("Custom field not found: {0}")]
    CustomFieldNotFound(String),
    #[error("Custom field key already exists: {0}")]
    CustomFieldKeyAlreadyExists(String),
    #[error("Custom field limit reached: {0}")]
    CustomFieldLimitReached(String),
    #[error("Invalid custom field value: {0}")]
    InvalidCustomFieldValue(String),
    #[error("Custom field reorder mismatch: {0}")]
    CustomFieldReorderMismatch(String),

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Password hashing failed: {0}")]
    PasswordHashingError(String),

    #[error("Infrastructure error: {0}")]
    InfrastructureError(String),

    #[error("Security block not found: {0}")]
    SecurityBlockNotFound(String),

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

    #[error("Email verification failed: {0}")]
    EmailVerificationError(String),

    #[error("Email not confirmed: {0}")]
    EmailNotConfirmed(String),

    #[error("Password reset failed: {0}")]
    PasswordResetError(String),

    #[error("Password change failed: {0}")]
    PasswordChangeError(String),

    #[error("MFA verification failed: {0}")]
    MfaError(String),
    #[error("MFA enrollment not found: {0}")]
    MfaEnrollmentNotFound(String),
    #[error("MFA enrollment conflict: {0}")]
    MfaEnrollmentConflict(String),

    #[error("Email provider not found: {0}")]
    EmailProviderNotFound(String),
    #[error("Email provider error: {0}")]
    EmailProviderError(String),

    #[error("Webhook subscription not found: {0}")]
    WebhookSubscriptionNotFound(String),

    #[error("External identity provider not found: {0}")]
    ExternalIdentityProviderNotFound(String),
    #[error("External identity provider already exists: {0}")]
    ExternalIdentityProviderAlreadyExists(String),
    #[error("External provider error: {0}")]
    ExternalProviderError(String),

    #[error("Email template error: {0}")]
    EmailTemplateError(String),
}
