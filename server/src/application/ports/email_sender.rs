use async_trait::async_trait;

use crate::domain::{entities::email_provider::EmailProvider, errors::DomainError};

// Outbound email is an infrastructure concern, but use cases decide *when* to
// send and *through which* provider — so the port lives in the application layer
// and the concrete transport (SMTP, or the dev log sender) is injected from
// infrastructure.
//
// The use case resolves which provider to use (the OAuth client's chosen one, or
// the tenant default) and passes it in. `None` means no provider is configured,
// in which case the transport falls back to a dev log sender.
#[async_trait]
pub trait EmailSender: Send + Sync {
    // Sends the self-registration confirmation email carrying the 6-digit
    // verification code the user types back into the confirm-email screen.
    async fn send_verification_code(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
        code: &str,
    ) -> Result<(), DomainError>;

    // Sends the "forgot password" email carrying the one-time reset link the user
    // clicks to choose a new password.
    async fn send_password_reset_link(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
        link: &str,
    ) -> Result<(), DomainError>;

    // Sends the sign-in second-factor email carrying the 6-digit MFA code the
    // user types back into the verify screen.
    async fn send_mfa_code(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
        code: &str,
    ) -> Result<(), DomainError>;

    // Sends a one-off diagnostic message so an admin can verify a provider's SMTP
    // credentials right after saving — surfacing the transport's exact error
    // (e.g. a 535 auth rejection) instead of waiting for a real flow to trigger
    // it. Unlike the flows above, a test deliberately sends even when the
    // provider is inactive, so credentials can be validated before going live.
    async fn send_test_email(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
    ) -> Result<(), DomainError>;
}
