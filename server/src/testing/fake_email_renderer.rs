use crate::{
    application::ports::email_renderer::{EmailRenderer, RenderedEmail},
    domain::errors::DomainError,
};

/// Test double for `EmailRenderer` that skips MJML compilation:
/// - `subject` and `html` get naive `{{placeholder}}` substitution, so rendering
///   tests (e.g. preview) can assert the substituted output;
/// - `text` is a deterministic `key=value` dump of the variables, so auth-flow
///   tests can read back the exact generated code/link the transport received
///   (the real MJML body is full of 6-digit hex colors, which would make
///   extracting a 6-digit code from rendered markup unreliable).
pub struct FakeEmailRenderer;

impl FakeEmailRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FakeEmailRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailRenderer for FakeEmailRenderer {
    fn render(
        &self,
        subject_template: &str,
        body_mjml: &str,
        vars: &[(String, String)],
    ) -> Result<RenderedEmail, DomainError> {
        let subject = substitute(subject_template, vars);
        let html = substitute(body_mjml, vars);
        let text = vars
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(RenderedEmail {
            subject,
            html,
            text,
        })
    }
}

/// Reads a variable's value back out of [`FakeEmailRenderer`]'s `text` dump.
pub fn var_from_text(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .map(|v| v.to_string())
}

fn substitute(input: &str, vars: &[(String, String)]) -> String {
    let mut out = input.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
        out = out.replace(&format!("{{{{ {key} }}}}"), value);
    }
    out
}
