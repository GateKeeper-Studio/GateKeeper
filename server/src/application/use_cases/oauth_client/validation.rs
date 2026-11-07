use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    application::use_cases::webhook_subscription::validation::validate_outbound_url,
    domain::{
        entities::oauth_client::{ClientType, HeroSide, LoginAppearance, LoginFont, LoginLayout},
        errors::DomainError,
        locale::is_supported_locale,
    },
};

pub fn parse_client_type(value: &str) -> Result<ClientType, DomainError> {
    match value {
        "public" => Ok(ClientType::Public),
        "confidential" => Ok(ClientType::Confidential),
        other => Err(DomainError::InvalidRequest(format!(
            "invalid client_type: {other}"
        ))),
    }
}

// SECURITY: redirect URIs are the #1 defense against open-redirect / code theft.
// We require them to be absolute and unambiguous BEFORE they are ever stored, so
// the exact-match check at /authorize has something safe to compare against:
//   - must be absolute (have a scheme),
//   - must NOT contain a fragment (#...),
//   - plain http is only allowed for loopback (localhost / 127.0.0.1) — native
//     apps and local dev; everything else must be https (or a custom scheme).
pub fn validate_redirect_uris(uris: &[String]) -> Result<(), DomainError> {
    if uris.is_empty() {
        return Err(DomainError::InvalidRequest(
            "at least one redirect_uri is required".to_string(),
        ));
    }
    for uri in uris {
        validate_one("redirect_uri", uri)?;
    }
    Ok(())
}

// RP-Initiated Logout sends the browser to one of these after sign-out, so they get
// the same rules as redirect_uris. Unlike redirect_uris the list may be empty: the
// logout endpoint then answers without redirecting.
pub fn validate_post_logout_redirect_uris(uris: &[String]) -> Result<(), DomainError> {
    for uri in uris {
        validate_one("post_logout_redirect_uri", uri)?;
    }
    Ok(())
}

fn validate_one(label: &str, uri: &str) -> Result<(), DomainError> {
    let invalid = |msg: String| DomainError::InvalidRequest(msg);

    if uri.contains('#') {
        return Err(invalid(format!(
            "{label} must not contain a fragment: {uri}"
        )));
    }

    let Some((scheme, rest)) = uri.split_once("://") else {
        // Allow custom schemes of the form "scheme:/path" (RFC 8252 native apps),
        // but they still must have a scheme and a colon.
        if uri.contains(':') && !uri.starts_with(':') {
            return Ok(());
        }
        return Err(invalid(format!("{label} must be an absolute URI: {uri}")));
    };

    match scheme {
        "https" => Ok(()),
        "http" => {
            let host = rest.split(['/', ':', '?']).next().unwrap_or("");
            if host == "localhost" || host == "127.0.0.1" || host == "[::1]" {
                Ok(())
            } else {
                Err(invalid(format!(
                    "plain http {label} is only allowed for loopback: {uri}"
                )))
            }
        }
        // Custom application scheme (e.g. com.example.app://callback) — permitted.
        _ => Ok(()),
    }
}

pub fn require_non_empty(label: &str, items: &[String]) -> Result<(), DomainError> {
    if items.is_empty() {
        Err(DomainError::InvalidRequest(format!(
            "{label} cannot be empty"
        )))
    } else {
        Ok(())
    }
}

// Trims an optional free-text field and collapses an empty/whitespace value to
// None, so a blank legal-URI input from the admin form is stored as "unset"
// (fall back to the platform pages) rather than an empty string.
pub fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

// SECURITY: tos_uri / policy_uri are rendered as `href`s on the hosted login
// screen. Only allow absolute http(s) URLs so an admin can't (accidentally or
// otherwise) inject a `javascript:` / `data:` URI that would execute in the
// auth UI. None is always valid - it means "fall back to the platform page".
pub fn validate_legal_uri(label: &str, uri: &Option<String>) -> Result<(), DomainError> {
    let Some(uri) = uri else {
        return Ok(());
    };
    let lower = uri.to_ascii_lowercase();
    let Some((scheme, rest)) = lower.split_once("://") else {
        return Err(DomainError::InvalidRequest(format!(
            "{label} must be an absolute http(s) URL"
        )));
    };
    let host = rest.split(['/', ':', '?']).next().unwrap_or("");
    if (scheme == "https" || scheme == "http") && !host.is_empty() {
        Ok(())
    } else {
        Err(DomainError::InvalidRequest(format!(
            "{label} must be an absolute http(s) URL"
        )))
    }
}

// SECURITY: initiate_login_uri is a NAVIGATION target — the hosted login page sends
// the browser here when the OAuth state/PKCE it was handed has expired and only the RP
// can mint a fresh one. That makes it strictly more dangerous than a rendered `href`,
// so it gets the redirect_uri bar rather than the legal-URI one:
//
//   * https only, except plain http on loopback for local development;
//   * no custom application schemes — unlike a redirect_uri this is followed by the
//     browser from our own origin, and handing that to `myapp://…` buys nothing;
//   * no fragment, which a browser would silently drop on navigation anyway.
//
// The open-redirect defence is that this value comes from the client registration and
// never from a query parameter. Validating it here is defence in depth against a
// compromised or careless admin, not the primary control.
pub fn validate_initiate_login_uri(uri: &Option<String>) -> Result<(), DomainError> {
    let Some(uri) = uri else {
        return Ok(());
    };

    let invalid = || {
        DomainError::InvalidRequest(
            "initiate_login_uri must be an absolute https URL (http allowed for loopback only)"
                .to_string(),
        )
    };

    if uri.contains('#') {
        return Err(invalid());
    }

    let lower = uri.to_ascii_lowercase();
    let Some((scheme, rest)) = lower.split_once("://") else {
        return Err(invalid());
    };
    let host = rest.split(['/', ':', '?']).next().unwrap_or("");
    if host.is_empty() {
        return Err(invalid());
    }

    match scheme {
        "https" => Ok(()),
        "http" if host == "localhost" || host == "127.0.0.1" || host == "[::1]" => Ok(()),
        _ => Err(invalid()),
    }
}

// Raw, un-validated login-appearance payload as it arrives from the admin API.
// Every field is a plain string keyed camelCase to match the dashboard, so the
// same object the dashboard reads back can be sent straight back on save.
// `validate_login_appearance` is the ONLY way to turn this into the domain value
// object — nothing untrusted reaches the entity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginAppearanceInput {
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub hero_position: Option<String>,
    #[serde(default)]
    pub logo_uri: Option<String>,
    #[serde(default)]
    pub background_image_uri: Option<String>,
    #[serde(default)]
    pub heading: Option<HashMap<String, String>>,
    #[serde(default)]
    pub subtext: Option<HashMap<String, String>>,
    #[serde(default)]
    pub primary_color: Option<String>,
    #[serde(default)]
    pub accent_color: Option<String>,
    #[serde(default)]
    pub border_radius: Option<String>,
    #[serde(default)]
    pub font_family: Option<String>,
}

// OIDC Back-Channel Logout: the RP endpoint our dispatcher POSTs a logout_token to.
// It is an outbound request from inside our network to an admin-supplied address,
// so it passes the same SSRF checks as a webhook URL (https only, no private or
// metadata addresses, no fragment, no credentials). The spec also forbids a
// fragment; the shared check already refuses one.
pub fn validate_backchannel_logout_uri(
    uri: &Option<String>,
    allow_private_targets: bool,
) -> Result<(), DomainError> {
    match uri {
        Some(uri) => validate_outbound_url("backchannel_logout_uri", uri, allow_private_targets),
        None => Ok(()),
    }
}

// SECURITY: this is the trust boundary for per-client login-page customization.
// It converts the raw admin input into a fully-validated domain value object,
// rejecting anything that could smuggle code into the hosted login UI:
//   - layout / hero_position / font_family must be whitelisted enum keys,
//   - logo / background image must be absolute http(s) URLs (no javascript:/data:),
//   - colors must be plain hex (so they're safe to feed a CSS custom property),
//   - border_radius must be a bounded number with a fixed rem/px unit,
//   - heading / subtext are length-limited plain text (the UI renders them escaped).
// None in → None out ("use the default look").
pub fn validate_login_appearance(
    input: Option<LoginAppearanceInput>,
) -> Result<Option<LoginAppearance>, DomainError> {
    let Some(input) = input else {
        return Ok(None);
    };

    // Layout is mandatory whenever an appearance object is supplied.
    let layout_key = input.layout.as_deref().map(str::trim).unwrap_or("");
    let layout = LoginLayout::from_key(layout_key).ok_or_else(|| {
        DomainError::InvalidRequest(format!("invalid login layout: '{layout_key}'"))
    })?;

    let hero_position = match input.hero_position.as_deref().map(str::trim) {
        None | Some("") => HeroSide::Left,
        Some(key) => HeroSide::from_key(key).ok_or_else(|| {
            DomainError::InvalidRequest(format!("invalid hero_position: '{key}'"))
        })?,
    };

    let font_family =
        match input.font_family.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(key) => Some(LoginFont::from_key(key).ok_or_else(|| {
                DomainError::InvalidRequest(format!("invalid font_family: '{key}'"))
            })?),
        };

    let logo_uri = normalize_optional(input.logo_uri);
    validate_legal_uri("logo_uri", &logo_uri)?;
    let background_image_uri = normalize_optional(input.background_image_uri);
    validate_legal_uri("background_image_uri", &background_image_uri)?;

    let primary_color = normalize_optional(input.primary_color);
    validate_hex_color("primary_color", &primary_color)?;
    let accent_color = normalize_optional(input.accent_color);
    validate_hex_color("accent_color", &accent_color)?;

    let border_radius = normalize_optional(input.border_radius);
    validate_border_radius("border_radius", &border_radius)?;

    let heading = validate_localized_text("heading", input.heading, 120)?;
    let subtext = validate_localized_text("subtext", input.subtext, 300)?;

    Ok(Some(LoginAppearance {
        layout,
        hero_position,
        logo_uri,
        background_image_uri,
        heading,
        subtext,
        primary_color,
        accent_color,
        border_radius,
        font_family,
    }))
}

// A #RGB or #RRGGBB value. Restricting to hex means the value is safe to inject as
// a CSS custom property — it cannot contain `;`, `}`, `url(`, quotes, or anything
// else that could break out of the CSS context.
fn validate_hex_color(label: &str, value: &Option<String>) -> Result<(), DomainError> {
    let Some(v) = value else {
        return Ok(());
    };
    let valid = (v.len() == 4 || v.len() == 7)
        && v.starts_with('#')
        && v[1..].chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(DomainError::InvalidRequest(format!(
            "{label} must be a hex color like #1a2b3c"
        )))
    }
}

// A bounded number followed by a fixed unit (`rem` or `px`). We validate the
// character set ourselves before parsing so f32::parse can't accept `inf`, `NaN`,
// or exponent forms like `1e9` — only plain decimals in [0, 64] get through.
fn validate_border_radius(label: &str, value: &Option<String>) -> Result<(), DomainError> {
    let Some(v) = value else {
        return Ok(());
    };
    let invalid = || {
        DomainError::InvalidRequest(format!(
            "{label} must be a number from 0 to 64 followed by rem or px"
        ))
    };
    let num = v
        .strip_suffix("rem")
        .or_else(|| v.strip_suffix("px"))
        .ok_or_else(invalid)?;

    let mut dots = 0usize;
    let charset_ok = !num.is_empty()
        && num.chars().all(|c| {
            if c == '.' {
                dots += 1;
                true
            } else {
                c.is_ascii_digit()
            }
        })
        && dots <= 1;
    if !charset_ok {
        return Err(invalid());
    }
    match num.parse::<f32>() {
        Ok(n) if (0.0..=64.0).contains(&n) => Ok(()),
        _ => Err(invalid()),
    }
}

// Validates a per-language free-text map (heading/subtext): trims every value,
// drops blanks, rejects any key outside SUPPORTED_LOCALES, length-limits each
// remaining value (measured in characters, not bytes — the login UI renders
// each as a text node, so Svelte escapes it and there is no markup to sanitize
// here), and collapses an empty result to None (no language ended up with text).
fn validate_localized_text(
    label: &str,
    map: Option<HashMap<String, String>>,
    max: usize,
) -> Result<Option<HashMap<String, String>>, DomainError> {
    let Some(map) = map else {
        return Ok(None);
    };

    let mut result = HashMap::new();
    for (locale, value) in map {
        if !is_supported_locale(&locale) {
            return Err(DomainError::InvalidRequest(format!(
                "{label}.{locale}: unsupported locale"
            )));
        }
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > max {
            return Err(DomainError::InvalidRequest(format!(
                "{label}.{locale} must be at most {max} characters"
            )));
        }
        result.insert(locale, value);
    }

    Ok(if result.is_empty() {
        None
    } else {
        Some(result)
    })
}

// SECURITY/CORRECTNESS: this is the trust boundary for which languages a
// client's hosted login page offers end users, and which one it defaults to.
// Every language must be one this deployment actually has messages for
// (SUPPORTED_LOCALES), the set must be non-empty, and default_language must be
// a member of it — otherwise the hosted page could be told to default to a
// language it never offers a switcher option for.
pub fn validate_client_languages(
    allowed_languages: Vec<String>,
    default_language: String,
) -> Result<(Vec<String>, String), DomainError> {
    let mut langs = allowed_languages;
    langs.sort();
    langs.dedup();

    if langs.is_empty() || langs.iter().any(|l| !is_supported_locale(l)) {
        return Err(DomainError::InvalidRequest(
            "allowed_languages must be a non-empty subset of the supported locales".to_string(),
        ));
    }
    if !langs.contains(&default_language) {
        return Err(DomainError::InvalidRequest(
            "default_language must be one of allowed_languages".to_string(),
        ));
    }

    Ok((langs, default_language))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{LoginAppearanceInput, validate_client_languages, validate_login_appearance};
    use crate::domain::{
        entities::oauth_client::{HeroSide, LoginFont, LoginLayout},
        errors::DomainError,
    };

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // A minimal valid input (layout is the only mandatory field).
    fn base() -> LoginAppearanceInput {
        LoginAppearanceInput {
            layout: Some("centered".into()),
            ..Default::default()
        }
    }

    fn assert_rejected(input: LoginAppearanceInput) {
        assert!(matches!(
            validate_login_appearance(Some(input)),
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[test]
    fn none_input_yields_none() {
        assert!(validate_login_appearance(None).unwrap().is_none());
    }

    #[test]
    fn accepts_a_fully_specified_valid_appearance() {
        let input = LoginAppearanceInput {
            layout: Some("split_image".into()),
            hero_position: Some("right".into()),
            logo_uri: Some("https://cdn.example.com/logo.png".into()),
            background_image_uri: Some("https://cdn.example.com/bg.jpg".into()),
            heading: Some(map(&[("en", "Welcome back")])),
            subtext: Some(map(&[("en", "Sign in to continue")])),
            primary_color: Some("#1a2b3c".into()),
            accent_color: Some("#fff".into()),
            border_radius: Some("0.75rem".into()),
            font_family: Some("inter".into()),
        };
        let a = validate_login_appearance(Some(input)).unwrap().unwrap();
        assert_eq!(a.layout, LoginLayout::SplitImage);
        assert_eq!(a.hero_position, HeroSide::Right);
        assert_eq!(a.primary_color.as_deref(), Some("#1a2b3c"));
        assert_eq!(a.accent_color.as_deref(), Some("#fff"));
        assert_eq!(a.font_family, Some(LoginFont::Inter));
    }

    #[test]
    fn requires_a_valid_layout_when_object_present() {
        // Missing layout is rejected...
        assert_rejected(LoginAppearanceInput::default());
        // ...as is an unknown layout key.
        assert_rejected(LoginAppearanceInput {
            layout: Some("evil".into()),
            ..Default::default()
        });
    }

    #[test]
    fn rejects_unknown_font_and_hero_position() {
        assert_rejected(LoginAppearanceInput {
            font_family: Some("comic".into()),
            ..base()
        });
        assert_rejected(LoginAppearanceInput {
            hero_position: Some("diagonal".into()),
            ..base()
        });
    }

    #[test]
    fn rejects_non_http_image_urls() {
        assert_rejected(LoginAppearanceInput {
            logo_uri: Some("javascript:alert(1)".into()),
            ..base()
        });
        assert_rejected(LoginAppearanceInput {
            background_image_uri: Some("data:text/html,owned".into()),
            ..base()
        });
    }

    #[test]
    fn rejects_colors_that_are_not_plain_hex() {
        // A CSS-breakout attempt and an ordinary non-hex word are both refused.
        assert_rejected(LoginAppearanceInput {
            primary_color: Some("#fff;}html{background:url(x)".into()),
            ..base()
        });
        assert_rejected(LoginAppearanceInput {
            primary_color: Some("red".into()),
            ..base()
        });
    }

    #[test]
    fn rejects_bad_border_radius_forms() {
        for bad in ["10", "10em", "1e9rem", "infrem", "NaNpx", "-1rem", "999rem"] {
            assert_rejected(LoginAppearanceInput {
                border_radius: Some(bad.into()),
                ..base()
            });
        }
    }

    #[test]
    fn accepts_good_border_radius() {
        for good in ["0rem", "0.5rem", "12px", "2rem"] {
            let input = LoginAppearanceInput {
                border_radius: Some(good.into()),
                ..base()
            };
            assert!(
                validate_login_appearance(Some(input)).is_ok(),
                "expected {good} to pass"
            );
        }
    }

    #[test]
    fn rejects_overlong_text() {
        assert_rejected(LoginAppearanceInput {
            heading: Some(map(&[("en", &"x".repeat(121))])),
            ..base()
        });
        assert_rejected(LoginAppearanceInput {
            subtext: Some(map(&[("en", &"y".repeat(301))])),
            ..base()
        });
    }

    #[test]
    fn rejects_unsupported_locale_key_in_heading() {
        assert_rejected(LoginAppearanceInput {
            heading: Some(map(&[("fr", "Bienvenue")])),
            ..base()
        });
    }

    #[test]
    fn keeps_script_like_heading_verbatim_as_data() {
        // The validator length-limits text but does NOT sanitize it: the login UI
        // renders it as an escaped text node, so a script-looking heading is stored
        // as data and can never execute.
        let input = LoginAppearanceInput {
            heading: Some(map(&[("en", "<script>alert(1)</script>")])),
            ..base()
        };
        let a = validate_login_appearance(Some(input)).unwrap().unwrap();
        assert_eq!(
            a.heading.unwrap().get("en").map(String::as_str),
            Some("<script>alert(1)</script>")
        );
    }

    #[test]
    fn blank_optionals_normalize_to_none() {
        let input = LoginAppearanceInput {
            logo_uri: Some("   ".into()),
            heading: Some(map(&[("en", "")])),
            primary_color: Some("  ".into()),
            ..base()
        };
        let a = validate_login_appearance(Some(input)).unwrap().unwrap();
        assert!(a.logo_uri.is_none());
        assert!(a.heading.is_none());
        assert!(a.primary_color.is_none());
    }

    #[test]
    fn validate_client_languages_accepts_valid_set() {
        let (langs, default) =
            validate_client_languages(vec!["en".into(), "es".into()], "en".into()).unwrap();
        assert_eq!(langs, vec!["en".to_string(), "es".to_string()]);
        assert_eq!(default, "en");
    }

    #[test]
    fn validate_client_languages_rejects_empty_set() {
        assert!(matches!(
            validate_client_languages(vec![], "en".into()),
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[test]
    fn validate_client_languages_rejects_unsupported_locale() {
        assert!(matches!(
            validate_client_languages(vec!["en".into(), "fr".into()], "en".into()),
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[test]
    fn validate_client_languages_rejects_default_not_in_allowed() {
        assert!(matches!(
            validate_client_languages(vec!["en".into()], "es".into()),
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[test]
    fn validate_client_languages_dedupes() {
        let (langs, _) =
            validate_client_languages(vec!["en".into(), "en".into()], "en".into()).unwrap();
        assert_eq!(langs, vec!["en".to_string()]);
    }
}
