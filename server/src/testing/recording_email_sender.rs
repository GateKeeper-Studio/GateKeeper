use std::sync::Mutex;

use async_trait::async_trait;

use crate::{
    application::ports::email_sender::EmailSender,
    domain::{entities::email_provider::EmailProvider, errors::DomainError},
};

/// A rendered email captured by the test double.
#[derive(Debug, Clone)]
pub struct SentEmail {
    pub to: String,
    pub subject: String,
    pub html: String,
    pub text: String,
}

// Test double that records every email it is asked to send so tests can assert
// the recipient and the rendered content (which embeds the generated code/link).
pub struct RecordingEmailSender {
    pub sent: Mutex<Vec<SentEmail>>,
    pub test_emails: Mutex<Vec<String>>, // recipients of test emails
}

impl RecordingEmailSender {
    pub fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            test_emails: Mutex::new(Vec::new()),
        }
    }

    /// Convenience for assertions: the recipients of every non-test email sent.
    pub fn recipients(&self) -> Vec<String> {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.to.clone())
            .collect()
    }
}

impl Default for RecordingEmailSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailSender for RecordingEmailSender {
    async fn send(
        &self,
        _provider: Option<&EmailProvider>,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), DomainError> {
        self.sent.lock().unwrap().push(SentEmail {
            to: to.to_string(),
            subject: subject.to_string(),
            html: html_body.to_string(),
            text: text_body.to_string(),
        });
        Ok(())
    }

    async fn send_test_email(
        &self,
        _provider: Option<&EmailProvider>,
        to: &str,
    ) -> Result<(), DomainError> {
        self.test_emails.lock().unwrap().push(to.to_string());
        Ok(())
    }
}
