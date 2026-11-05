use async_trait::async_trait;

use crate::{
    application::ports::email_sender::EmailSender,
    domain::{entities::email_provider::EmailProvider, errors::DomainError},
};

// Development email transport: instead of delivering mail, it logs the code via
// `tracing` so you can copy it from the server output and finish the flow.
//
// NON-PRODUCTION: this writes the raw, single-use verification code to the logs.
// A real `SmtpEmailSender` / provider client must replace it behind the same
// `EmailSender` port before going live, and production logs must never contain
// codes.
pub struct LogEmailSender;

impl LogEmailSender {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LogEmailSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailSender for LogEmailSender {
    async fn send(
        &self,
        _provider: Option<&EmailProvider>,
        to: &str,
        subject: &str,
        _html_body: &str,
        text_body: &str,
    ) -> Result<(), DomainError> {
        tracing::info!(
          recipient = %to,
          subject = %subject,
          "[dev email] (replace LogEmailSender with a real transport in production): {}",
          text_body
        );
        Ok(())
    }

    async fn send_test_email(
        &self,
        _provider: Option<&EmailProvider>,
        to: &str,
    ) -> Result<(), DomainError> {
        tracing::info!(
          recipient = %to,
          "[dev email] test email (LogEmailSender does not deliver real mail — configure an SMTP provider to actually send)"
        );
        Ok(())
    }
}
