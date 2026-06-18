use std::sync::Mutex;

use async_trait::async_trait;

use crate::{
    application::ports::email_sender::EmailSender,
    domain::{entities::email_provider::EmailProvider, errors::DomainError},
};

// Test double that records every email it is asked to send so tests can assert
// the recipient and the code/link that was generated.
pub struct RecordingEmailSender {
    pub sent: Mutex<Vec<(String, String)>>, // (recipient, code)
    pub reset_links: Mutex<Vec<(String, String)>>, // (recipient, reset link)
    pub mfa_codes: Mutex<Vec<(String, String)>>, // (recipient, MFA sign-in code)
    pub test_emails: Mutex<Vec<String>>,    // recipients of test emails
}

impl RecordingEmailSender {
    pub fn new() -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            reset_links: Mutex::new(Vec::new()),
            mfa_codes: Mutex::new(Vec::new()),
            test_emails: Mutex::new(Vec::new()),
        }
    }
}

impl Default for RecordingEmailSender {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailSender for RecordingEmailSender {
    async fn send_verification_code(
        &self,
        _provider: Option<&EmailProvider>,
        to: &str,
        code: &str,
    ) -> Result<(), DomainError> {
        self.sent
            .lock()
            .unwrap()
            .push((to.to_string(), code.to_string()));
        Ok(())
    }

    async fn send_password_reset_link(
        &self,
        _provider: Option<&EmailProvider>,
        to: &str,
        link: &str,
    ) -> Result<(), DomainError> {
        self.reset_links
            .lock()
            .unwrap()
            .push((to.to_string(), link.to_string()));
        Ok(())
    }

    async fn send_mfa_code(
        &self,
        _provider: Option<&EmailProvider>,
        to: &str,
        code: &str,
    ) -> Result<(), DomainError> {
        self.mfa_codes
            .lock()
            .unwrap()
            .push((to.to_string(), code.to_string()));
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
