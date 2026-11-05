use std::sync::Arc;

use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart},
    transport::smtp::authentication::Credentials,
};

use crate::{
    application::ports::{email_sender::EmailSender, secret_cipher::SecretCipher},
    domain::{
        entities::email_provider::{EmailProvider, SmtpEncryption},
        errors::DomainError,
    },
};

// Production email transport. Delivers through the tenant's configured SMTP
// provider (Gmail, Outlook, any SMTP host); the provider's password is decrypted
// from its at-rest ciphertext only here, at send time. When no provider is
// resolved (`None`) it falls back to logging — so dev/unconfigured flows keep
// working without delivering real mail.
pub struct SmtpEmailSender {
    secret_cipher: Arc<dyn SecretCipher>,
}

impl SmtpEmailSender {
    pub fn new(secret_cipher: Arc<dyn SecretCipher>) -> Self {
        Self { secret_cipher }
    }

    // Decrypts the password and builds the from-mailbox + transport shared by every
    // delivery path. The `is_active` gate lives in the callers, not here, so the
    // test-email path can deliberately validate an inactive provider's credentials
    // before it goes live.
    fn prepare(
        &self,
        provider: &EmailProvider,
    ) -> Result<(Mailbox, AsyncSmtpTransport<Tokio1Executor>), DomainError> {
        let password = self.secret_cipher.decrypt(provider.password_encrypted())?;
        let from = build_mailbox(provider.from_email(), provider.from_name())?;
        let transport = build_transport(provider, password)?;
        Ok((from, transport))
    }

    // Delivers a `multipart/alternative` message (plain-text + HTML) so modern
    // clients render the HTML while text-only clients still get a readable body.
    async fn deliver(
        &self,
        provider: &EmailProvider,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), DomainError> {
        let (from, transport) = self.prepare(provider)?;
        let to_mailbox = parse_recipient(to)?;

        let email = Message::builder()
            .from(from)
            .to(to_mailbox)
            .subject(subject)
            .multipart(MultiPart::alternative_plain_html(
                text_body.to_string(),
                html_body.to_string(),
            ))
            .map_err(|e| DomainError::EmailProviderError(format!("failed to build email: {e}")))?;

        transport.send(email).await.map_err(|e| {
            tracing::error!("SMTP send failed: {e}");
            DomainError::EmailProviderError(format!("failed to send email: {e}"))
        })?;

        Ok(())
    }

    // Delivers a plain-text-only message (used by the diagnostic test email).
    async fn deliver_text(
        &self,
        provider: &EmailProvider,
        to: &str,
        subject: &str,
        text_body: &str,
    ) -> Result<(), DomainError> {
        let (from, transport) = self.prepare(provider)?;
        let to_mailbox = parse_recipient(to)?;

        let email = Message::builder()
            .from(from)
            .to(to_mailbox)
            .subject(subject)
            .body(text_body.to_string())
            .map_err(|e| DomainError::EmailProviderError(format!("failed to build email: {e}")))?;

        transport.send(email).await.map_err(|e| {
            tracing::error!("SMTP send failed: {e}");
            DomainError::EmailProviderError(format!("failed to send email: {e}"))
        })?;

        Ok(())
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), DomainError> {
        let Some(provider) = provider else {
            // No SMTP provider configured for this client or tenant — log so local/dev
            // flows still surface the content. Replace by configuring a provider.
            tracing::info!(recipient = %to, subject = %subject, "[dev email] no SMTP provider configured: {}", text_body);
            return Ok(());
        };

        if !provider.is_active() {
            return Err(DomainError::EmailProviderError(
                "the selected email provider is inactive".to_string(),
            ));
        }

        self.deliver(provider, to, subject, html_body, text_body)
            .await
    }

    async fn send_test_email(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
    ) -> Result<(), DomainError> {
        let Some(provider) = provider else {
            return Err(DomainError::EmailProviderError(
                "no email provider configured to test".to_string(),
            ));
        };
        let body = "This is a test email from GateKeeper.\n\nIf you received it, this SMTP provider's credentials and connection are working.";
        // Bypasses the `is_active` gate on purpose — see `deliver_text`.
        self.deliver_text(provider, to, "GateKeeper SMTP test", body)
            .await
    }
}

// Parses a recipient address into a mailbox, mapping failures to a domain error.
fn parse_recipient(to: &str) -> Result<Mailbox, DomainError> {
    to.parse()
        .map_err(|_| DomainError::EmailProviderError(format!("invalid recipient address: {to}")))
}

// "Name <email>" when a display name is set, otherwise the bare address.
fn build_mailbox(email: &str, name: Option<&str>) -> Result<Mailbox, DomainError> {
    let raw = match name {
        Some(name) if !name.trim().is_empty() => format!("{name} <{email}>"),
        _ => email.to_string(),
    };
    raw.parse()
        .map_err(|_| DomainError::EmailProviderError(format!("invalid from address: {email}")))
}

fn build_transport(
    provider: &EmailProvider,
    password: String,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, DomainError> {
    let host = provider.host();
    let builder = match provider.encryption() {
        SmtpEncryption::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .map_err(map_smtp_build_error)?,
        SmtpEncryption::Tls => {
            AsyncSmtpTransport::<Tokio1Executor>::relay(host).map_err(map_smtp_build_error)?
        }
        // Plaintext (or local) relay — dev only.
        SmtpEncryption::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host),
    };

    let port = u16::try_from(provider.port())
        .map_err(|_| DomainError::EmailProviderError("invalid SMTP port".to_string()))?;

    Ok(builder
        .port(port)
        .credentials(Credentials::new(provider.username().to_string(), password))
        .build())
}

fn map_smtp_build_error(e: lettre::transport::smtp::Error) -> DomainError {
    tracing::error!("SMTP transport build failed: {e}");
    DomainError::EmailProviderError(format!("failed to connect to SMTP host: {e}"))
}
