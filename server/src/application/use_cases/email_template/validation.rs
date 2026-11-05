use std::collections::HashSet;

use crate::domain::{entities::email_template::EmailTemplateType, errors::DomainError};

/// Validates an admin-submitted template before it is stored. This is the
/// edit-boundary half of the injection defense (the render path also scrubs the
/// compiled HTML and the dashboard previews inside a sandboxed iframe):
///
/// 1. reject raw passthrough (`<script>`, `<mj-raw>`) outright;
/// 2. allow only the situation's placeholder keys — every other `{{key}}` is an
///    error, so a template can never reference unexpected data.
pub fn validate(
    template_type: EmailTemplateType,
    subject: &str,
    body_mjml: &str,
) -> Result<(), DomainError> {
    let lowered = body_mjml.to_lowercase();
    if lowered.contains("<script") {
        return Err(DomainError::EmailTemplateError(
            "inline <script> is not allowed in email templates".to_string(),
        ));
    }
    if lowered.contains("<mj-raw") {
        return Err(DomainError::EmailTemplateError(
            "<mj-raw> is not allowed in email templates".to_string(),
        ));
    }

    let allowed: HashSet<&str> = template_type.variables().iter().copied().collect();
    for key in placeholders(subject)
        .into_iter()
        .chain(placeholders(body_mjml))
    {
        if !allowed.contains(key.as_str()) {
            return Err(DomainError::EmailTemplateError(format!(
                "unknown placeholder {{{{{key}}}}}; allowed: {}",
                template_type.variables().join(", ")
            )));
        }
    }

    Ok(())
}

/// Extracts the trimmed key of every `{{ ... }}` placeholder in `input`. A
/// regex-free scan keeps the dependency surface small. Unterminated `{{` is
/// ignored (treated as literal text).
pub fn placeholders(input: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(end) = input[i + 2..].find("}}") {
                let key = input[i + 2..i + 2 + end].trim();
                if !key.is_empty() {
                    keys.push(key.to_string());
                }
                i = i + 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
    keys
}
