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
    /// Magic link login: a one-time link that signs the user in.
    MagicLink,
    /// Admin password reset: the single-use temporary password that lets the user
    /// sign in once to choose a new one.
    TemporaryPassword,
    /// Account created by an admin: the welcome email with the single-use temporary
    /// password the user signs in with to choose their own.
    AccountCreated,
    /// The 6-digit code an account without a password confirms a sensitive change
    /// with (removing a sign-in method, adding a passkey, deleting the account).
    StepUpCode,
}

impl EmailTemplateType {
    /// Every known situation, in display order. Drives the dashboard list and any
    /// "iterate all templates" logic.
    pub const ALL: &'static [EmailTemplateType] = &[
        EmailTemplateType::EmailVerification,
        EmailTemplateType::PasswordReset,
        EmailTemplateType::MfaCode,
        EmailTemplateType::MagicLink,
        EmailTemplateType::TemporaryPassword,
        EmailTemplateType::AccountCreated,
        EmailTemplateType::StepUpCode,
    ];

    /// Stored value in the `template_type` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            EmailTemplateType::EmailVerification => "email_verification",
            EmailTemplateType::PasswordReset => "password_reset",
            EmailTemplateType::MfaCode => "mfa_code",
            EmailTemplateType::MagicLink => "magic_link",
            EmailTemplateType::TemporaryPassword => "temporary_password",
            EmailTemplateType::AccountCreated => "account_created",
            EmailTemplateType::StepUpCode => "step_up_code",
        }
    }

    /// Parse the stored column value back into the enum.
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "email_verification" => Some(EmailTemplateType::EmailVerification),
            "password_reset" => Some(EmailTemplateType::PasswordReset),
            "mfa_code" => Some(EmailTemplateType::MfaCode),
            "magic_link" => Some(EmailTemplateType::MagicLink),
            "temporary_password" => Some(EmailTemplateType::TemporaryPassword),
            "account_created" => Some(EmailTemplateType::AccountCreated),
            "step_up_code" => Some(EmailTemplateType::StepUpCode),
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
            EmailTemplateType::MagicLink => "Magic sign-in link",
            EmailTemplateType::TemporaryPassword => "Temporary password",
            EmailTemplateType::AccountCreated => "New account",
            EmailTemplateType::StepUpCode => "Confirmation code for a sensitive change",
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
            EmailTemplateType::MagicLink => {
                "Sent when a user asks to sign in with a one-time link instead of a password."
            }
            EmailTemplateType::TemporaryPassword => {
                "Sent when an admin resets a user's password, with a temporary password that \
                 signs in once so the user can choose a new one."
            }
            EmailTemplateType::AccountCreated => {
                "Sent when an admin creates a user, with a temporary password that signs in \
                 once so the user can choose their own."
            }
            EmailTemplateType::StepUpCode => {
                "Sent when a user without a password or passkey confirms a sensitive change \
                 to their account, such as deleting it or adding a passkey."
            }
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
            EmailTemplateType::MfaCode | EmailTemplateType::StepUpCode => {
                &["code", "app_name", "expiry", "support_email"]
            }
            EmailTemplateType::MagicLink => &["link", "app_name", "expiry", "support_email"],
            EmailTemplateType::TemporaryPassword | EmailTemplateType::AccountCreated => {
                &["password", "app_name", "expiry", "support_email"]
            }
        }
    }

    /// Built-in default subject (a plain-text template; may use placeholders).
    pub fn default_subject(&self) -> &'static str {
        match self {
            EmailTemplateType::EmailVerification => "Confirm your email - {{app_name}}",
            EmailTemplateType::PasswordReset => "Reset your password - {{app_name}}",
            EmailTemplateType::MfaCode => "Your {{app_name}} sign-in code",
            EmailTemplateType::MagicLink => "Sign in to {{app_name}}",
            EmailTemplateType::TemporaryPassword => "Your temporary {{app_name}} password",
            EmailTemplateType::AccountCreated => "Your new {{app_name}} account",
            EmailTemplateType::StepUpCode => "Confirm a change to your {{app_name}} account",
        }
    }

    /// Built-in default MJML body served when a tenant has not customized the
    /// template. Single source of truth for the default markup.
    pub fn default_body_mjml(&self) -> &'static str {
        match self {
            EmailTemplateType::EmailVerification => DEFAULT_EMAIL_VERIFICATION_MJML,
            EmailTemplateType::PasswordReset => DEFAULT_PASSWORD_RESET_MJML,
            EmailTemplateType::MfaCode => DEFAULT_MFA_CODE_MJML,
            EmailTemplateType::MagicLink => DEFAULT_MAGIC_LINK_MJML,
            EmailTemplateType::TemporaryPassword => DEFAULT_TEMPORARY_PASSWORD_MJML,
            EmailTemplateType::AccountCreated => DEFAULT_ACCOUNT_CREATED_MJML,
            EmailTemplateType::StepUpCode => DEFAULT_STEP_UP_CODE_MJML,
        }
    }

    /// Fake values for every variable, used to render the dashboard preview.
    pub fn sample_context(&self) -> Vec<(String, String)> {
        self.variables()
            .iter()
            .map(|key| {
                let value = match *key {
                    "code" => "123456",
                    "password" => "K7QM-4TZP-WXRB-9HNC",
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

const DEFAULT_MAGIC_LINK_MJML: &str = r##"<mjml>
  <mj-body background-color="#f4f4f7">
    <mj-section padding="24px 0 0 0">
      <mj-column>
        <mj-text align="center" font-size="20px" font-weight="bold" color="#111827">{{app_name}}</mj-text>
      </mj-column>
    </mj-section>
    <mj-section background-color="#ffffff" border-radius="8px" padding="32px">
      <mj-column>
        <mj-text font-size="18px" font-weight="bold" color="#111827">Sign in to {{app_name}}</mj-text>
        <mj-text font-size="14px" color="#374151" line-height="22px">Click the button below to sign in. Open it in the same browser where you asked for the link. It works once and expires in {{expiry}}.</mj-text>
        <mj-button background-color="#111827" color="#ffffff" border-radius="6px" href="{{link}}" padding="16px 0">Sign in</mj-button>
        <mj-text font-size="12px" color="#6b7280" line-height="20px">If you did not try to sign in, you can safely ignore this email: the link does nothing in any other browser. Need help? Contact {{support_email}}.</mj-text>
      </mj-column>
    </mj-section>
  </mj-body>
</mjml>
"##;

// No link on purpose: the user signs in through their app as usual, so the email
// teaches nobody to follow links that ask for a password.
const DEFAULT_TEMPORARY_PASSWORD_MJML: &str = r##"<mjml>
  <mj-body background-color="#f4f4f7">
    <mj-section padding="24px 0 0 0">
      <mj-column>
        <mj-text align="center" font-size="20px" font-weight="bold" color="#111827">{{app_name}}</mj-text>
      </mj-column>
    </mj-section>
    <mj-section background-color="#ffffff" border-radius="8px" padding="32px">
      <mj-column>
        <mj-text font-size="18px" font-weight="bold" color="#111827">Your password was reset</mj-text>
        <mj-text font-size="14px" color="#374151" line-height="22px">An administrator reset your password. Your old password no longer works, and you were signed out everywhere. Sign in with this temporary password, then choose a new one:</mj-text>
        <mj-text align="center" font-size="22px" font-weight="bold" letter-spacing="2px" font-family="Menlo, Consolas, monospace" color="#111827" padding="16px 0">{{password}}</mj-text>
        <mj-text font-size="14px" color="#374151" line-height="22px">It works for one sign-in only and expires in {{expiry}}. Nobody from {{app_name}} will ever ask you for it.</mj-text>
        <mj-text font-size="12px" color="#6b7280" line-height="20px">If you did not expect this email, contact {{support_email}}.</mj-text>
      </mj-column>
    </mj-section>
  </mj-body>
</mjml>
"##;

// Like the reset email: no link, the user signs in through their app as usual.
const DEFAULT_ACCOUNT_CREATED_MJML: &str = r##"<mjml>
  <mj-body background-color="#f4f4f7">
    <mj-section padding="24px 0 0 0">
      <mj-column>
        <mj-text align="center" font-size="20px" font-weight="bold" color="#111827">{{app_name}}</mj-text>
      </mj-column>
    </mj-section>
    <mj-section background-color="#ffffff" border-radius="8px" padding="32px">
      <mj-column>
        <mj-text font-size="18px" font-weight="bold" color="#111827">Welcome to {{app_name}}</mj-text>
        <mj-text font-size="14px" color="#374151" line-height="22px">An administrator created an account for you with this email address. Sign in with this temporary password, then choose your own:</mj-text>
        <mj-text align="center" font-size="22px" font-weight="bold" letter-spacing="2px" font-family="Menlo, Consolas, monospace" color="#111827" padding="16px 0">{{password}}</mj-text>
        <mj-text font-size="14px" color="#374151" line-height="22px">It works for one sign-in only and expires in {{expiry}}. Nobody from {{app_name}} will ever ask you for it.</mj-text>
        <mj-text font-size="12px" color="#6b7280" line-height="20px">If you did not expect this email, contact {{support_email}}.</mj-text>
      </mj-column>
    </mj-section>
  </mj-body>
</mjml>
"##;

// Doubles as a warning: whoever asked for it is changing how the account signs in.
const DEFAULT_STEP_UP_CODE_MJML: &str = r##"<mjml>
  <mj-body background-color="#f4f4f7">
    <mj-section padding="24px 0 0 0">
      <mj-column>
        <mj-text align="center" font-size="20px" font-weight="bold" color="#111827">{{app_name}}</mj-text>
      </mj-column>
    </mj-section>
    <mj-section background-color="#ffffff" border-radius="8px" padding="32px">
      <mj-column>
        <mj-text font-size="18px" font-weight="bold" color="#111827">Confirm it's you</mj-text>
        <mj-text font-size="14px" color="#374151" line-height="22px">Someone signed in to your account is making a sensitive change: removing a sign-in method, adding a passkey or deleting the account. Enter this code to confirm it. It expires in {{expiry}}.</mj-text>
        <mj-text align="center" font-size="28px" font-weight="bold" letter-spacing="6px" color="#111827" padding="16px 0">{{code}}</mj-text>
        <mj-text font-size="12px" color="#6b7280" line-height="20px">If this wasn't you, don't share the code: sign out of all your devices and contact {{support_email}}.</mj-text>
      </mj-column>
    </mj-section>
  </mj-body>
</mjml>
"##;
