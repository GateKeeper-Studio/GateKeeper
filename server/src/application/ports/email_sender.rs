use async_trait::async_trait;

use crate::domain::{entities::email_provider::EmailProvider, errors::DomainError};

// Outbound email transport. Use cases decide *when* to send and *through which*
// provider — so the port lives in the application layer and the concrete
// transport (SMTP, or the dev log sender) is injected from infrastructure.
//
// This is a pure transport: the subject and the already-rendered HTML/plain-text
// bodies are produced upstream by `TemplatedEmailSender` (template lookup +
// `EmailRenderer`). The caller resolves which provider to use (the OAuth client's
// chosen one, or the tenant default) and passes it in. `None` means no provider
// is configured, in which case the transport falls back to a dev log sender.
#[async_trait]
pub trait EmailSender: Send + Sync {
    /// Delivers a rendered transactional email as `multipart/alternative`
    /// (plain-text + HTML).
    async fn send(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), DomainError>;

    // Sends a one-off diagnostic message so an admin can verify a provider's SMTP
    // credentials right after saving — surfacing the transport's exact error
    // (e.g. a 535 auth rejection) instead of waiting for a real flow to trigger
    // it. Unlike `send`, a test deliberately sends even when the provider is
    // inactive, so credentials can be validated before going live.
    async fn send_test_email(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
    ) -> Result<(), DomainError>;
}
