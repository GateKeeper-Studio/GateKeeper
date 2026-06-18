use crate::domain::{entities::oauth_client::ClientType, errors::DomainError};

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
        validate_one(uri)?;
    }
    Ok(())
}

fn validate_one(uri: &str) -> Result<(), DomainError> {
    let invalid = |msg: String| DomainError::InvalidRequest(msg);

    if uri.contains('#') {
        return Err(invalid(format!(
            "redirect_uri must not contain a fragment: {uri}"
        )));
    }

    let Some((scheme, rest)) = uri.split_once("://") else {
        // Allow custom schemes of the form "scheme:/path" (RFC 8252 native apps),
        // but they still must have a scheme and a colon.
        if uri.contains(':') && !uri.starts_with(':') {
            return Ok(());
        }
        return Err(invalid(format!(
            "redirect_uri must be an absolute URI: {uri}"
        )));
    };

    match scheme {
        "https" => Ok(()),
        "http" => {
            let host = rest.split(['/', ':', '?']).next().unwrap_or("");
            if host == "localhost" || host == "127.0.0.1" || host == "[::1]" {
                Ok(())
            } else {
                Err(invalid(format!(
                    "plain http redirect_uri is only allowed for loopback: {uri}"
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
