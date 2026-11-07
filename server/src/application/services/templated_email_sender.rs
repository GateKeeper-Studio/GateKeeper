use std::sync::Arc;

use uuid::Uuid;

use crate::{
    application::ports::{email_renderer::EmailRenderer, email_sender::EmailSender},
    domain::{
        entities::{email_provider::EmailProvider, email_template::EmailTemplateType},
        errors::DomainError,
        repositories::email_template_repository::EmailTemplateRepository,
    },
};

/// Orchestrates "look up the tenant's template (or the built-in default), render
/// it with the standard variables, and hand the rendered subject/HTML/text to the
/// transport". The authentication flows depend on this instead of the raw
/// `EmailSender`, so adding template support did not change how they decide *when*
/// to send — only that the body now comes from an editable template.
pub struct TemplatedEmailSender {
    template_repository: Arc<dyn EmailTemplateRepository>,
    renderer: Arc<dyn EmailRenderer>,
    sender: Arc<dyn EmailSender>,
}

impl TemplatedEmailSender {
    pub fn new(
        template_repository: Arc<dyn EmailTemplateRepository>,
        renderer: Arc<dyn EmailRenderer>,
        sender: Arc<dyn EmailSender>,
    ) -> Self {
        Self {
            template_repository,
            renderer,
            sender,
        }
    }

    /// Resolves the template (custom row or registry default), renders it with
    /// `vars`, and delivers it through the underlying transport.
    async fn send_templated(
        &self,
        tenant_id: Uuid,
        template_type: EmailTemplateType,
        provider: Option<&EmailProvider>,
        to: &str,
        vars: Vec<(String, String)>,
    ) -> Result<(), DomainError> {
        let (subject_template, body_mjml) = match self
            .template_repository
            .find_by_tenant_and_type(tenant_id, template_type)
            .await?
        {
            Some(template) => (
                template.subject().to_string(),
                template.body_mjml().to_string(),
            ),
            None => (
                template_type.default_subject().to_string(),
                template_type.default_body_mjml().to_string(),
            ),
        };

        let rendered = self.renderer.render(&subject_template, &body_mjml, &vars)?;

        self.sender
            .send(
                provider,
                to,
                &rendered.subject,
                &rendered.html,
                &rendered.text,
            )
            .await
    }

    /// Self-registration confirmation carrying the verification code.
    pub async fn send_verification_code(
        &self,
        tenant_id: Uuid,
        provider: Option<&EmailProvider>,
        to: &str,
        code: &str,
        app_name: &str,
        expiry: &str,
    ) -> Result<(), DomainError> {
        let vars = vec![
            ("code".to_string(), code.to_string()),
            ("app_name".to_string(), app_name.to_string()),
            ("expiry".to_string(), expiry.to_string()),
            ("support_email".to_string(), support_email(provider)),
        ];
        self.send_templated(
            tenant_id,
            EmailTemplateType::EmailVerification,
            provider,
            to,
            vars,
        )
        .await
    }

    /// "Forgot password" email carrying the one-time reset link.
    pub async fn send_password_reset_link(
        &self,
        tenant_id: Uuid,
        provider: Option<&EmailProvider>,
        to: &str,
        link: &str,
        app_name: &str,
        expiry: &str,
    ) -> Result<(), DomainError> {
        let vars = vec![
            ("link".to_string(), link.to_string()),
            ("app_name".to_string(), app_name.to_string()),
            ("expiry".to_string(), expiry.to_string()),
            ("support_email".to_string(), support_email(provider)),
        ];
        self.send_templated(
            tenant_id,
            EmailTemplateType::PasswordReset,
            provider,
            to,
            vars,
        )
        .await
    }

    /// Magic link login: the one-time link that signs the user in.
    pub async fn send_magic_link(
        &self,
        tenant_id: Uuid,
        provider: Option<&EmailProvider>,
        to: &str,
        link: &str,
        app_name: &str,
        expiry: &str,
    ) -> Result<(), DomainError> {
        let vars = vec![
            ("link".to_string(), link.to_string()),
            ("app_name".to_string(), app_name.to_string()),
            ("expiry".to_string(), expiry.to_string()),
            ("support_email".to_string(), support_email(provider)),
        ];
        self.send_templated(tenant_id, EmailTemplateType::MagicLink, provider, to, vars)
            .await
    }

    /// Sign-in second-factor email carrying the MFA code.
    pub async fn send_mfa_code(
        &self,
        tenant_id: Uuid,
        provider: Option<&EmailProvider>,
        to: &str,
        code: &str,
        app_name: &str,
        expiry: &str,
    ) -> Result<(), DomainError> {
        let vars = vec![
            ("code".to_string(), code.to_string()),
            ("app_name".to_string(), app_name.to_string()),
            ("expiry".to_string(), expiry.to_string()),
            ("support_email".to_string(), support_email(provider)),
        ];
        self.send_templated(tenant_id, EmailTemplateType::MfaCode, provider, to, vars)
            .await
    }

    /// Pass-through to the transport's diagnostic test email (no template).
    pub async fn send_test_email(
        &self,
        provider: Option<&EmailProvider>,
        to: &str,
    ) -> Result<(), DomainError> {
        self.sender.send_test_email(provider, to).await
    }
}

/// The reply-to / support address shown in templates: the provider's from-email
/// when one is configured, otherwise a neutral placeholder.
fn support_email(provider: Option<&EmailProvider>) -> String {
    provider
        .map(|p| p.from_email().to_string())
        .unwrap_or_else(|| "support@example.com".to_string())
}
