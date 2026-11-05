use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::errors::DomainError;

#[derive(Debug, Deserialize)]
pub struct Command {
    // tenant_id is injected from the URL path, so skip deserializing it from the body.
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub tenant_id: Uuid,
    #[serde(skip_deserializing, default = "Uuid::nil")]
    pub organization_id: Uuid,

    // The desired appearance, or null to revert to the default theme.
    #[serde(default)]
    pub appearance: Option<AppearanceInput>,
}

// Raw, un-validated appearance payload as it arrives from the admin dashboard.
// camelCase to match the object the dashboard reads back, so the same shape can be
// sent straight back on save. `validate_appearance` is the ONLY way to turn this
// into the stored JSON — nothing untrusted reaches the entity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceInput {
    #[serde(default)]
    pub primary_color: Option<String>,
    #[serde(default)]
    pub radius: Option<String>,
    #[serde(default)]
    pub default_mode: Option<String>,
}

const RADII: [&str; 4] = ["none", "sm", "md", "lg"];
const MODES: [&str; 3] = ["light", "dark", "system"];

// SECURITY: this is the trust boundary for per-organization dashboard theming. It
// converts the raw admin input into a normalized JSON blob whose values are safe to
// feed CSS custom properties:
//   - primary_color must be plain hex (so it cannot break out of the CSS context),
//   - radius / default_mode must be whitelisted enum keys.
// None in => None out ("use the default theme").
pub fn validate_appearance(input: Option<AppearanceInput>) -> Result<Option<Value>, DomainError> {
    let Some(input) = input else {
        return Ok(None);
    };

    let primary_color = normalize(input.primary_color)
        .ok_or_else(|| DomainError::InvalidRequest("primary_color is required".to_string()))?;
    validate_hex_color(&primary_color)?;

    let radius = normalize(input.radius).unwrap_or_else(|| "md".to_string());
    if !RADII.contains(&radius.as_str()) {
        return Err(DomainError::InvalidRequest(format!(
            "invalid radius: '{radius}'"
        )));
    }

    let default_mode = normalize(input.default_mode).unwrap_or_else(|| "system".to_string());
    if !MODES.contains(&default_mode.as_str()) {
        return Err(DomainError::InvalidRequest(format!(
            "invalid default_mode: '{default_mode}'"
        )));
    }

    Ok(Some(json!({
        "primaryColor": primary_color,
        "radius": radius,
        "defaultMode": default_mode,
    })))
}

fn normalize(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

// A #RGB or #RRGGBB value. Restricting to hex means the value is safe to inject as a
// CSS custom property — it cannot contain `;`, `}`, `url(`, quotes, or anything else
// that could break out of the CSS context.
fn validate_hex_color(value: &str) -> Result<(), DomainError> {
    let valid = (value.len() == 4 || value.len() == 7)
        && value.starts_with('#')
        && value[1..].chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(DomainError::InvalidRequest(
            "primary_color must be a hex color like #1a2b3c".to_string(),
        ))
    }
}
