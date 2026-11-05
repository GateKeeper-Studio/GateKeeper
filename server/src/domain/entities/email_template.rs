use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The transactional-email "situation" a template is for. This enum is the
/// extensibility core of the email-templates feature: adding a new situation is
/// just a new variant plus its metadata below (label, default subject/body, and
/// the allow-list of placeholders). The CRUD use cases, the dashboard list, and
/// the preview/render path are all registry-driven and pick it up automatically.
///
/// Persisted as a short string in `template_type` and exposed in URLs via the
/// same value (also accepting a hyphenated slug for convenience).
///
/// `Default` exists only so command structs can carry the type as a
/// `#[serde(skip_deserializing, default)]` field that the handler overwrites from
/// the path — the value is never relied upon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EmailTemplateType {
    /// Self-registration confirmation carrying the 6-digit verification code.
    #[default]
    EmailVerification,
    /// "Forgot password" email carrying the one-time reset link.
    PasswordReset,
    /// Sign-in second-factor email carrying the 6-digit MFA code.
    MfaCode,
}

impl EmailTemplateType {
    /// Every known situation, in display order. Drives the dashboard list and any
    /// "iterate all templates" logic.
    pub const ALL: &'static [EmailTemplateType] = &[
        EmailTemplateType::EmailVerification,
        EmailTemplateType::PasswordReset,
        EmailTemplateType::MfaCode,
    ];

    /// Stored value in the `template_type` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            EmailTemplateType::EmailVerification => "email_verification",
            EmailTemplateType::PasswordReset => "password_reset",
            EmailTemplateType::MfaCode => "mfa_code",
        }
    }

    /// Parse the stored column value back into the enum.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "email_verification" => Some(EmailTemplateType::EmailVerification),
            "password_reset" => Some(EmailTemplateType::PasswordReset),
            "mfa_code" => Some(EmailTemplateType::MfaCode),
            _ => None,
        }
    }

    /// Parse the URL slug used by the admin routes. Accepts both the stored
    /// underscore form and a hyphenated variant.
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::from_str(&slug.replace('-', "_"))
    }

    /// Human-readable name for the dashboard list.
    pub fn label(&self) -> &'static str {
        match self {
            EmailTemplateType::EmailVerification => "Email confirmation",
            EmailTemplateType::PasswordReset => "Password reset",
            EmailTemplateType::MfaCode => "MFA sign-in code",
        }
    }

    /// One-line explanation of when this email is sent.
    pub fn description(&self) -> &'static str {
        match self {
            EmailTemplateType::EmailVerification => {
                "Sent during sign-up with the email verification code."
            }
            EmailTemplateType::PasswordReset => "Sent when a user requests a password-reset link.",
            EmailTemplateType::MfaCode => "Sent during sign-in as the email second-factor code.",
        }
    }

    /// The allow-list of `{{placeholder}}` keys this situation may reference.
    /// `update_email_template` rejects any other key — there is no
    /// arbitrary/Turing-complete templating, only substitution of these.
    pub fn variables(&self) -> &'static [&'static str] {
        match self {
            EmailTemplateType::EmailVerification => {
                &["code", "app_name", "expiry", "support_email"]
            }
            EmailTemplateType::PasswordReset => &["link", "app_name", "expiry", "support_email"],
            EmailTemplateType::MfaCode => &["code", "app_name", "expiry", "support_email"],
        }
    }

    /// Built-in default subject (a plain-text template; may use placeholders).
    pub fn default_subject(&self) -> &'static str {
        match self {
            EmailTemplateType::EmailVerification => "Confirm your email - {{app_name}}",
            EmailTemplateType::PasswordReset => "Reset your password - {{app_name}}",
            EmailTemplateType::MfaCode => "Your {{app_name}} sign-in code",
        }
    }

    /// Built-in default MJML body served when a tenant has not customized the
    /// template. Single source of truth for the default markup.
    pub fn default_body_mjml(&self) -> &'static str {
        match self {
            EmailTemplateType::EmailVerification => DEFAULT_EMAIL_VERIFICATION_MJML,
            EmailTemplateType::PasswordReset => DEFAULT_PASSWORD_RESET_MJML,
            EmailTemplateType::MfaCode => DEFAULT_MFA_CODE_MJML,
        }
    }

    /// Fake values for every variable, used to render the dashboard preview.
    pub fn sample_context(&self) -> Vec<(String, String)> {
        self.variables()
            .iter()
            .map(|key| {
                let value = match *key {
                    "code" => "123456",
                    "link" => "https://app.example.com/auth/change-password?token=sample",
                    "app_name" => "Example App",
                    "expiry" => "15 minutes",
                    "support_email" => "support@example.com",
                    _ => "sample",
                };
                ((*key).to_string(), value.to_string())
            })
            .collect()
    }
}

/// A tenant's editable email template for one situation, stored as MJML source.
#[derive(Debug, Clone)]
pub struct EmailTemplate {
    tenant_id: Uuid,
    template_type: EmailTemplateType,
    subject: String,
    body_mjml: String,
    updated_at: DateTime<Utc>,
}

impl EmailTemplate {
    pub fn new(
        tenant_id: Uuid,
        template_type: EmailTemplateType,
        subject: String,
        body_mjml: String,
    ) -> Self {
        EmailTemplate {
            tenant_id,
            template_type,
            subject,
            body_mjml,
            updated_at: Utc::now(),
        }
    }

    pub fn reconstruct(
        tenant_id: Uuid,
        template_type: EmailTemplateType,
        subject: String,
        body_mjml: String,
        updated_at: DateTime<Utc>,
    ) -> Self {
        EmailTemplate {
            tenant_id,
            template_type,
            subject,
            body_mjml,
            updated_at,
        }
    }

    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }
    pub fn template_type(&self) -> EmailTemplateType {
        self.template_type
    }
    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn body_mjml(&self) -> &str {
        &self.body_mjml
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

// ── Built-in default MJML templates ─────────────────────────────────────────────
// Plain-ASCII MJML so it compiles cleanly and renders Outlook-safe table HTML.

const DEFAULT_EMAIL_VERIFICATION_MJML: &str = r##"<mjml>
  <mj-body background-color="#f4f4f7">
    <mj-section padding="24px 0 0 0">
      <mj-column>
        <mj-text align="center" font-size="20px" font-weight="bold" color="#111827">{{app_name}}</mj-text>
      </mj-column>
    </mj-section>
    <mj-section background-color="#ffffff" border-radius="8px" padding="32px">
      <mj-column>
        <mj-text font-size="18px" font-weight="bold" color="#111827">Confirm your email</mj-text>
        <mj-text font-size="14px" color="#374151" line-height="22px">Use the verification code below to finish setting up your account. It expires in {{expiry}}.</mj-text>
        <mj-text align="center" font-size="32px" font-weight="bold" letter-spacing="6px" color="#111827" padding="16px 0">{{code}}</mj-text>
        <mj-text font-size="12px" color="#6b7280" line-height="20px">If you did not request this, you can safely ignore this email. Need help? Contact {{support_email}}.</mj-text>
      </mj-column>
    </mj-section>
  </mj-body>
</mjml>
"##;

const DEFAULT_PASSWORD_RESET_MJML: &str = r##"<mjml>
  <mj-body background-color="#f4f4f7">
    <mj-section padding="24px 0 0 0">
      <mj-column>
        <mj-text align="center" font-size="20px" font-weight="bold" color="#111827">{{app_name}}</mj-text>
      </mj-column>
    </mj-section>
    <mj-section background-color="#ffffff" border-radius="8px" padding="32px">
      <mj-column>
        <mj-text font-size="18px" font-weight="bold" color="#111827">Reset your password</mj-text>
        <mj-text font-size="14px" color="#374151" line-height="22px">We received a request to reset your password. Click the button below to choose a new one. This link expires in {{expiry}}.</mj-text>
        <mj-button background-color="#111827" color="#ffffff" border-radius="6px" href="{{link}}" padding="16px 0">Reset password</mj-button>
        <mj-text font-size="12px" color="#6b7280" line-height="20px">If you did not request this, you can safely ignore this email. Need help? Contact {{support_email}}.</mj-text>
      </mj-column>
    </mj-section>
  </mj-body>
</mjml>
"##;

const DEFAULT_MFA_CODE_MJML: &str = r##"<mjml>
  <mj-body background-color="#f4f4f7">
    <mj-section padding="24px 0 0 0">
      <mj-column>
        <mj-text align="center" font-size="20px" font-weight="bold" color="#111827">{{app_name}}</mj-text>
      </mj-column>
    </mj-section>
    <mj-section background-color="#ffffff" border-radius="8px" padding="32px">
      <mj-column>
        <mj-text font-size="18px" font-weight="bold" color="#111827">Your sign-in code</mj-text>
        <mj-text font-size="14px" color="#374151" line-height="22px">Enter the code below to finish signing in. It expires in {{expiry}}.</mj-text>
        <mj-text align="center" font-size="32px" font-weight="bold" letter-spacing="6px" color="#111827" padding="16px 0">{{code}}</mj-text>
        <mj-text font-size="12px" color="#6b7280" line-height="20px">If you did not try to sign in, change your password immediately. Need help? Contact {{support_email}}.</mj-text>
      </mj-column>
    </mj-section>
  </mj-body>
</mjml>
"##;
