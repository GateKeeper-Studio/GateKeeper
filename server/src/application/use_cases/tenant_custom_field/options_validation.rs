use std::collections::HashMap;

use crate::domain::{
    entities::custom_field_definition::CustomFieldType, errors::DomainError,
    locale::is_supported_locale,
};

const MAX_OPTIONS: usize = 50;
const MAX_OPTION_LEN: usize = 100;

// Shared by create_tenant_custom_field and update_tenant_custom_field_by_id:
// validates a per-language text map (`label`/`description`). Trims every
// value, drops empty entries, rejects any key that isn't a supported locale,
// and enforces a per-value length cap. `required_locale` is `Some(tenant's
// default_language)` for `label` (every field must have at least its own
// tenant's default-language text) and `None` for `description` (entirely
// optional, even in the default language).
pub fn validate_localized_text(
    label: &str,
    map: HashMap<String, String>,
    max_len: usize,
    required_locale: Option<&str>,
) -> Result<HashMap<String, String>, DomainError> {
    let mut cleaned = HashMap::new();
    for (locale, value) in map {
        if !is_supported_locale(&locale) {
            return Err(DomainError::Validation(format!(
                "{label}: unsupported locale '{locale}'"
            )));
        }
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > max_len {
            return Err(DomainError::Validation(format!(
                "{label}: text for '{locale}' must be at most {max_len} characters"
            )));
        }
        cleaned.insert(locale, value);
    }

    if let Some(required) = required_locale {
        if !cleaned.contains_key(required) {
            return Err(DomainError::Validation(format!(
                "{label} must include the tenant's default language ('{required}')"
            )));
        }
    }

    Ok(cleaned)
}

// Shared by create_tenant_custom_field and update_tenant_custom_field_by_id:
// a `select` field must declare 1-50 ordered choices, each a per-language
// text map that includes at least the tenant's default language
// (`required_locale`); any other field type simply has no options, regardless
// of what a caller might send (stray options for a non-select type are never
// persisted).
//
// Known limitation: uniqueness is only checked on each entry's
// `required_locale` value (the one key every entry is guaranteed to have) —
// two options that happen to share the same translation in a *non-default*
// locale, while differing in the default locale, are not caught. Fixing this
// would require every option to be fully translated into every locale before
// it could be validated at all, which is a stricter requirement than this
// feature asks for.
pub fn validate_localized_options(
    field_type: CustomFieldType,
    options: Option<Vec<HashMap<String, String>>>,
    required_locale: &str,
) -> Result<Option<Vec<HashMap<String, String>>>, DomainError> {
    if field_type != CustomFieldType::Select {
        return Ok(None);
    }

    let options = options.unwrap_or_default();
    if options.is_empty() || options.len() > MAX_OPTIONS {
        return Err(DomainError::Validation(format!(
            "a select field must declare between 1 and {MAX_OPTIONS} options"
        )));
    }

    let mut cleaned = Vec::with_capacity(options.len());
    let mut required_values = Vec::with_capacity(options.len());
    for entry in options {
        let validated =
            validate_localized_text("option", entry, MAX_OPTION_LEN, Some(required_locale))?;
        required_values.push(validated.get(required_locale).cloned().unwrap_or_default());
        cleaned.push(validated);
    }

    let mut deduped = required_values.clone();
    deduped.sort();
    deduped.dedup();
    if deduped.len() != required_values.len() {
        return Err(DomainError::Validation(
            "options must not contain duplicates".to_string(),
        ));
    }

    Ok(Some(cleaned))
}
