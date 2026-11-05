use crate::domain::errors::DomainError;

/// The product of rendering an email template: a substituted subject plus the
/// HTML and plain-text bodies of a `multipart/alternative` message.
#[derive(Debug, Clone)]
pub struct RenderedEmail {
    pub subject: String,
    pub html: String,
    pub text: String,
}

/// Compiles an admin-authored template (subject + MJML body) into a ready-to-send
/// email. Owned by the application layer; the concrete implementation lives in
/// infrastructure (MJML compile via `mrml`, post-compile scrub, plain-text
/// derivation) so this layer stays free of those crates.
///
/// `vars` is the substitution context — only the situation's allow-listed keys.
/// Implementations HTML-escape values before substitution and strip active
/// content from the compiled HTML; callers must still validate the allow-list at
/// the edit boundary.
pub trait EmailRenderer: Send + Sync {
    fn render(
        &self,
        subject_template: &str,
        body_mjml: &str,
        vars: &[(String, String)],
    ) -> Result<RenderedEmail, DomainError>;
}
