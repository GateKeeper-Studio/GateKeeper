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
    #[error("Cannot delete the default tenant: {0}")]
    CannotDeleteDefaultTenant(String),

    #[error("Organization not found: {0}")]
    OrganizationNotFound(String),
    #[error("Organization already exists: {0}")]
    OrganizationAlreadyExists(String),

    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),
    #[error("Organization already has a subscription: {0}")]
    OrganizationAlreadyHasSubscription(String),
    #[error("Subscription plan not found: {0}")]
    SubscriptionPlanNotFound(String),
    #[error("Subscription plan already exists: {0}")]
    SubscriptionPlanAlreadyExists(String),
    #[error("Subscription plan is inactive: {0}")]
    SubscriptionPlanInactive(String),
    #[error("Subscription plan is in use: {0}")]
    SubscriptionPlanInUse(String),
    #[error("Subscription limit reached: {0}")]
    SubscriptionLimitReached(String),
    #[error("Plan downgrade not allowed: {0}")]
    PlanDowngradeNotAllowed(String),

    #[error("Group not found: {0}")]
    GroupNotFound(String),
    #[error("Group already exists: {0}")]
    GroupAlreadyExists(String),

    #[error("Role not found: {0}")]
    RoleNotFound(String),
    #[error("Role already exists: {0}")]
    RoleAlreadyExists(String),

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
}
