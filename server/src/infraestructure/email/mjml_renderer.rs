use std::collections::HashMap;

use lol_html::{RewriteStrSettings, element, errors::RewritingError, rewrite_str};
use mrml::prelude::render::RenderOptions;

use crate::{
    application::ports::email_renderer::{EmailRenderer, RenderedEmail},
    domain::errors::DomainError,
};

/// Production email renderer. Pipeline for the HTML part:
///   1. substitute allow-listed `{{placeholders}}` with HTML-escaped values;
///   2. compile MJML -> table-based, Outlook-safe HTML (`mrml`);
///   3. scrub the compiled HTML: drop `<script>`, event-handler attributes, and
///      dangerous URL schemes, while preserving tables, inline styles, and the
///      responsive `<style>` head block (`lol_html` targeted rewrite);
///   4. derive a plain-text alternative from the scrubbed HTML (`html2text`).
///
/// The subject is substituted separately with CR/LF stripped from values to
/// prevent header injection (and is never HTML).
pub struct MjmlEmailRenderer;

impl MjmlEmailRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MjmlEmailRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailRenderer for MjmlEmailRenderer {
    fn render(
        &self,
        subject_template: &str,
        body_mjml: &str,
        vars: &[(String, String)],
    ) -> Result<RenderedEmail, DomainError> {
        let subject = apply_placeholders(subject_template, vars, SubstitutionMode::SubjectText);
        let substituted_mjml = apply_placeholders(body_mjml, vars, SubstitutionMode::HtmlBody);

        let parsed = mrml::parse(&substituted_mjml)
            .map_err(|e| DomainError::Validation(format!("invalid MJML: {e}")))?;
        let html = parsed
            .element
            .render(&RenderOptions::default())
            .map_err(|e| DomainError::Validation(format!("failed to render MJML: {e}")))?;

        let html = scrub_html(&html)?;
        let text = html_to_text(&html);

        Ok(RenderedEmail {
            subject,
            html,
            text,
        })
    }
}

#[derive(Clone, Copy)]
enum SubstitutionMode {
    /// HTML-escape values (they land inside compiled HTML).
    HtmlBody,
    /// Strip CR/LF only (subject is plain text, not HTML; guards header injection).
    SubjectText,
}

/// Replaces every `{{ key }}` whose trimmed key is in `vars`, leaving unknown
/// placeholders as literal text. Regex-free to keep the dependency surface small.
fn apply_placeholders(input: &str, vars: &[(String, String)], mode: SubstitutionMode) -> String {
    let map: HashMap<&str, String> = vars
        .iter()
        .map(|(k, v)| {
            let value = match mode {
                SubstitutionMode::HtmlBody => html_escape(v),
                SubstitutionMode::SubjectText => strip_newlines(v),
            };
            (k.as_str(), value)
        })
        .collect();

    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < input.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(end_rel) = input[i + 2..].find("}}") {
                let key = input[i + 2..i + 2 + end_rel].trim();
                match map.get(key) {
                    Some(value) => out.push_str(value),
                    None => out.push_str(&input[i..i + 2 + end_rel + 2]),
                }
                i = i + 2 + end_rel + 2;
                continue;
            }
        }
        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn html_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

fn strip_newlines(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

/// Removes active content from compiled HTML while preserving layout. This is the
/// render-time half of the injection defense (the edit boundary also validates).
fn scrub_html(html: &str) -> Result<String, DomainError> {
    let settings = RewriteStrSettings::new()
        // Drop <script> entirely (element + content).
        .append_element_content_handler(element!("script", |el| {
            el.remove();
            Ok(())
        }))
        // Strip inline event handlers and neutralize dangerous URL schemes on
        // every element.
        .append_element_content_handler(element!("*", |el| {
            let handlers: Vec<String> = el
                .attributes()
                .iter()
                .map(|a| a.name())
                .filter(|name| name.starts_with("on"))
                .collect();
            for name in handlers {
                el.remove_attribute(&name);
            }
            for attr in ["href", "src", "background", "action"] {
                if let Some(value) = el.get_attribute(attr) {
                    if is_dangerous_url(&value) {
                        el.remove_attribute(attr);
                    }
                }
            }
            Ok(())
        }));

    rewrite_str(html, settings).map_err(|e: RewritingError| {
        DomainError::Validation(format!("failed to sanitize email HTML: {e}"))
    })
}

/// True for URL values whose scheme can execute script or smuggle content.
/// `data:` is allowed only for images (inline logos), blocked otherwise.
fn is_dangerous_url(value: &str) -> bool {
    let normalized: String = value
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect::<String>()
        .to_lowercase();

    normalized.starts_with("javascript:")
        || normalized.starts_with("vbscript:")
        || (normalized.starts_with("data:") && !normalized.starts_with("data:image/"))
}

/// Best-effort plain-text alternative; falls back to a tag strip if the HTML
/// renderer errors so a delivery is never blocked on the text part.
fn html_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 80).unwrap_or_else(|_| naive_strip_tags(html))
}

fn naive_strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::email_template::EmailTemplateType;

    #[test]
    fn renders_default_templates_to_table_html_and_text() {
        let renderer = MjmlEmailRenderer::new();
        for template_type in EmailTemplateType::ALL {
            let rendered = renderer
                .render(
                    template_type.default_subject(),
                    template_type.default_body_mjml(),
                    &template_type.sample_context(),
                )
                .unwrap();

            // MJML compiles to a table-based, Outlook-safe layout.
            assert!(rendered.html.contains("<table"));
            assert!(!rendered.text.trim().is_empty());
            // Placeholders were substituted, not left literal.
            assert!(!rendered.html.contains("{{"));
        }
    }

    #[test]
    fn html_escapes_substituted_values() {
        let renderer = MjmlEmailRenderer::new();
        let vars = vec![
            ("code".to_string(), "<b>x</b>".to_string()),
            ("app_name".to_string(), "App".to_string()),
            ("expiry".to_string(), "5 minutes".to_string()),
            ("support_email".to_string(), "s@e.com".to_string()),
        ];

        let rendered = renderer
            .render(
                "Hi {{app_name}}",
                EmailTemplateType::EmailVerification.default_body_mjml(),
                &vars,
            )
            .unwrap();

        assert_eq!(rendered.subject, "Hi App");
        // The value's tags were escaped, so they never become live markup.
        assert!(!rendered.html.contains("<b>x</b>"));
    }

    #[test]
    fn strips_newlines_from_subject_values() {
        let renderer = MjmlEmailRenderer::new();
        let vars = vec![("app_name".to_string(), "Evil\r\nBcc: x@y.com".to_string())];
        let rendered = renderer
            .render(
                "Subject {{app_name}}",
                "<mjml><mj-body></mj-body></mjml>",
                &vars,
            )
            .unwrap();
        assert!(!rendered.subject.contains('\n'));
        assert!(!rendered.subject.contains('\r'));
    }

    #[test]
    fn scrubs_script_and_dangerous_urls() {
        let dirty = r#"<html><body><a href="javascript:alert(1)" onclick="x()">hi</a><script>bad()</script></body></html>"#;
        let clean = scrub_html(dirty).unwrap();
        assert!(!clean.to_lowercase().contains("<script"));
        assert!(!clean.to_lowercase().contains("onclick"));
        assert!(!clean.to_lowercase().contains("javascript:"));
    }

    #[test]
    fn is_dangerous_url_detects_schemes() {
        assert!(is_dangerous_url("javascript:alert(1)"));
        assert!(is_dangerous_url("  JaVaScRiPt:alert(1)"));
        assert!(is_dangerous_url("data:text/html;base64,abc"));
        assert!(!is_dangerous_url("data:image/png;base64,abc"));
        assert!(!is_dangerous_url("https://example.com/x?a=1&b=2"));
        assert!(!is_dangerous_url("mailto:s@e.com"));
    }
}
