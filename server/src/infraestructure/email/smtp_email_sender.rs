use std::sync::Arc;

use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
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

    async fn send(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), DomainError> {
        let Some(provider) = provider else {
            // No SMTP provider configured for this client or tenant — log so local/dev
            // flows still surface the content. Replace by configuring a provider.
            tracing::info!(recipient = %to, subject = %subject, "[dev email] no SMTP provider configured: {}", body);
            return Ok(());
        };

        if !provider.is_active() {
            return Err(DomainError::EmailProviderError(
                "the selected email provider is inactive".to_string(),
            ));
        }

        self.deliver(provider, to, subject, body).await
    }

    // Decrypts the password, builds the message + transport, and delivers it.
    // The `is_active` gate lives in `send`, not here, so the test-email path can
    // deliberately validate an inactive provider's credentials before it goes live.
    async fn deliver(
        &self,
        provider: &EmailProvider,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), DomainError> {
        let password = self.secret_cipher.decrypt(provider.password_encrypted())?;

        let from = build_mailbox(provider.from_email(), provider.from_name())?;
        let to_mailbox: Mailbox = to.parse().map_err(|_| {
            DomainError::EmailProviderError(format!("invalid recipient address: {to}"))
        })?;

        let email = Message::builder()
            .from(from)
            .to(to_mailbox)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| DomainError::EmailProviderError(format!("failed to build email: {e}")))?;

        let transport = build_transport(provider, password)?;

        transport.send(email).await.map_err(|e| {
            tracing::error!("SMTP send failed: {e}");
            DomainError::EmailProviderError(format!("failed to send email: {e}"))
        })?;

        Ok(())
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send_verification_code(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
        code: &str,
    ) -> Result<(), DomainError> {
        let body = format!(
            "Your verification code is: {code}\n\nIt expires shortly. If you didn't request this, you can ignore this email."
        );
        self.send(provider, to, "Your verification code", &body)
            .await
    }

    async fn send_password_reset_link(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
        link: &str,
    ) -> Result<(), DomainError> {
        let body = format!(
            "We received a request to reset your password.\n\nReset it here: {link}\n\nIf you didn't request this, you can ignore this email."
        );
        self.send(provider, to, "Reset your password", &body).await
    }

    async fn send_mfa_code(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
        code: &str,
    ) -> Result<(), DomainError> {
        let body = format!(
            "Your sign-in verification code is: {code}\n\nIt expires shortly. If you didn't try to sign in, change your password immediately."
        );
        self.send(provider, to, "Your sign-in verification code", &body)
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
        // Bypasses the `is_active` gate on purpose — see `deliver`.
        self.deliver(provider, to, "GateKeeper SMTP test", body)
            .await
    }
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
