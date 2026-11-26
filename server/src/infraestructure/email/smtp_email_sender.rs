use std::sync::Arc;

use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart},
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
};
use std::net::IpAddr;

use crate::application::use_cases::webhook_subscription::validation::is_blocked_ip;

use crate::application::ports::secret_cipher::{SecretContext, SecretPurpose};
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
    // Development only: with no SMTP provider configured, write the message to the
    // log so the code or link in it can be copied. See `send`.
    log_undelivered_bodies: bool,
    // Whether an SMTP host may be a private, loopback or link-local address
    // (WEBHOOK_ALLOW_PRIVATE_TARGETS: development and self-hosted setups).
    allow_private_targets: bool,
}

impl SmtpEmailSender {
    pub fn new(
        secret_cipher: Arc<dyn SecretCipher>,
        log_undelivered_bodies: bool,
        allow_private_targets: bool,
    ) -> Self {
        Self {
            secret_cipher,
            log_undelivered_bodies,
            allow_private_targets,
        }
    }

    // Decrypts the password and builds the from-mailbox + transport shared by every
    // delivery path. The `is_active` gate lives in the callers, not here, so the
    // test-email path can deliberately validate an inactive provider's credentials
    // before it goes live.
    async fn prepare(
        &self,
        provider: &EmailProvider,
    ) -> Result<(Mailbox, AsyncSmtpTransport<Tokio1Executor>), DomainError> {
        let address = resolve_smtp_host(provider, self.allow_private_targets).await?;
        let password = self.secret_cipher.decrypt(
            provider.password_encrypted(),
            SecretContext::new(SecretPurpose::SmtpPassword, provider.tenant_id()),
        )?;
        let from = build_mailbox(provider.from_email(), provider.from_name())?;
        let transport = build_transport(provider, address, password)?;
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
        let (from, transport) = self.prepare(provider).await?;
        let to_mailbox = parse_recipient(to)?;

        let email = Message::builder()
            .from(from)
            .to(to_mailbox)
            .subject(subject)
            .multipart(MultiPart::alternative_plain_html(
                text_body.to_string(),
                html_body.to_string(),
            ))
            .map_err(|e| {
                tracing::warn!("building email failed: {e}");
                DomainError::Upstream("the email could not be built".to_string())
            })?;

        transport.send(email).await.map_err(|e| {
            tracing::error!("SMTP send failed: {e}");
            DomainError::Upstream(
                "the SMTP server did not accept the message — check the provider credentials"
                    .to_string(),
            )
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
        let (from, transport) = self.prepare(provider).await?;
        let to_mailbox = parse_recipient(to)?;

        let email = Message::builder()
            .from(from)
            .to(to_mailbox)
            .subject(subject)
            .body(text_body.to_string())
            .map_err(|e| {
                tracing::warn!("building email failed: {e}");
                DomainError::Upstream("the email could not be built".to_string())
            })?;

        transport.send(email).await.map_err(|e| {
            tracing::error!("SMTP send failed: {e}");
            DomainError::Upstream(
                "the SMTP server did not accept the message — check the provider credentials"
                    .to_string(),
            )
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
            // No SMTP provider configured for this client or tenant.
            //
            // SECURITY: the body carries the one-time code or the reset / sign-in
            // link. Outside development it must never reach the log: whoever can read
            // the logs could take over any account of a tenant with no SMTP provider.
            if self.log_undelivered_bodies {
                tracing::info!(recipient = %to, subject = %subject, "[dev email] no SMTP provider configured: {}", text_body);
            } else {
                tracing::warn!(
                    subject = %subject,
                    "email not sent: the tenant has no active SMTP provider configured"
                );
            }
            return Ok(());
        };

        if !provider.is_active() {
            return Err(DomainError::Validation(
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
            return Err(DomainError::Validation(
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
        .map_err(|_| DomainError::Validation(format!("invalid recipient address: {to}")))
}

// "Name <email>" when a display name is set, otherwise the bare address.
fn build_mailbox(email: &str, name: Option<&str>) -> Result<Mailbox, DomainError> {
    let raw = match name {
        Some(name) if !name.trim().is_empty() => format!("{name} <{email}>"),
        _ => email.to_string(),
    };
    raw.parse()
        .map_err(|_| DomainError::Validation(format!("invalid from address: {email}")))
}

// SECURITY: the SMTP host comes from a tenant admin, and the server connects to it
// from inside its own network. Without a check that is a way to reach internal
// services (a mail relay that trusts the private network, any port that answers).
// The name is resolved here, every address must be public, and the connection is
// then made to that vetted address (`build_transport`), so a name that changes its
// answer between the check and the connection (DNS rebinding) gains nothing.
async fn resolve_smtp_host(
    provider: &EmailProvider,
    allow_private_targets: bool,
) -> Result<IpAddr, DomainError> {
    let unreachable = || DomainError::Upstream("could not connect to the SMTP host".to_string());
    let host = provider.host().trim();
    let port = u16::try_from(provider.port())
        .map_err(|_| DomainError::Validation("invalid SMTP port".to_string()))?;

    let addresses: Vec<IpAddr> = match host.trim_matches(['[', ']']).parse::<IpAddr>() {
        Ok(ip) => vec![ip],
        Err(_) => tokio::net::lookup_host((host, port))
            .await
            .map_err(|e| {
                tracing::warn!("SMTP host lookup failed: {e}");
                unreachable()
            })?
            .map(|address| address.ip())
            .collect(),
    };
    if !allow_private_targets && addresses.iter().any(is_blocked_ip) {
        return Err(DomainError::Validation(
            "the SMTP host is not allowed: it must resolve to a public address".to_string(),
        ));
    }
    // IPv4 first: a host without IPv6 connectivity could not reach an AAAA answer.
    let address = addresses
        .iter()
        .find(|ip| ip.is_ipv4())
        .or_else(|| addresses.first())
        .copied()
        .ok_or_else(unreachable)?;

    // SECURITY: `encryption: none` sends the SMTP password and every message (codes,
    // reset links) in clear. That is acceptable on the same host or private network
    // (a local relay, a development mail catcher) and never across the internet.
    if *provider.encryption() == SmtpEncryption::None && !is_blocked_ip(&address) {
        return Err(DomainError::Validation(
            "unencrypted SMTP is only allowed for a local or private host: use starttls or tls"
                .to_string(),
        ));
    }
    Ok(address)
}

// Connects to `address` (already vetted) and keeps the provider's host name for the
// TLS handshake, so the certificate is still checked against the name.
fn build_transport(
    provider: &EmailProvider,
    address: IpAddr,
    password: String,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, DomainError> {
    let tls_parameters =
        || TlsParameters::new(provider.host().trim().to_string()).map_err(map_smtp_build_error);
    let tls = match provider.encryption() {
        SmtpEncryption::StartTls => Tls::Required(tls_parameters()?),
        SmtpEncryption::Tls => Tls::Wrapper(tls_parameters()?),
        // Plaintext (or local) relay — dev only.
        SmtpEncryption::None => Tls::None,
    };

    let port = u16::try_from(provider.port())
        .map_err(|_| DomainError::Validation("invalid SMTP port".to_string()))?;

    Ok(
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(address.to_string())
            .port(port)
            .tls(tls)
            .credentials(Credentials::new(provider.username().to_string(), password))
            .build(),
    )
}

fn map_smtp_build_error(e: lettre::transport::smtp::Error) -> DomainError {
    tracing::error!("SMTP transport build failed: {e}");
    DomainError::Upstream("could not connect to the SMTP host".to_string())
}
