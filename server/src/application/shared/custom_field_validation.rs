use chrono::NaiveDate;
use serde_json::{Map, Value};

use crate::{
    application::shared::localization::resolve_localized,
    domain::{entities::custom_field_definition::CustomFieldDefinition, errors::DomainError},
};

// Error messages need ONE representative string out of a per-language label
// map, but this validator has no tenant/locale context (it runs from
// sign-up and profile-edit paths that don't carry "which locale is the user
// looking at"). "en" is the app's base locale everywhere else (Paraglide's
// baseLocale) — used as both the preferred and fallback key here, so this
// just means "the English label, or whatever translation exists if not".
fn any_label(definition: &CustomFieldDefinition) -> &str {
    resolve_localized(definition.label(), "en", "en").unwrap_or_else(|| definition.field_key())
}

// Hard caps enforced regardless of what a tenant admin configured — bound
// storage/query cost and block JSON-bomb-style abuse. Independent of any
// per-field checks below.
const MAX_TEXT_VALUE_LEN: usize = 1000;
const MAX_TOTAL_PAYLOAD_BYTES: usize = 10 * 1024;

// SECURITY: this is the trust boundary for tenant-collected custom data (at
// sign-up and in later profile edits). It takes the tenant's own field
// definitions plus a raw, un-trusted JSON payload and returns either a fully
// validated value map or a rejection — nothing from `raw` reaches the stored
// profile except values that passed every check below:
//   - allow-list only: any key not matching an active field's `field_key` is
//     rejected outright (never silently dropped, never persisted),
//   - required active fields must be present and non-empty,
//   - every value is type-checked against its field's declared `field_type`,
//   - `select` values must exact-match one of the field's own `options` (a
//     client can't smuggle an arbitrary string through a "constrained" field),
//   - the output is rebuilt field-by-field from validated data, never a
//     pass-through of `raw` — defense in depth even if the allow-list check
//     above had a bug,
//   - the total serialized size is capped independently of the per-field caps.
//
// Assumes full-replace semantics: the caller always resends the complete
// current value map (matching how first_name/last_name are already a full-PUT,
// not a PATCH) — a missing optional key means "no answer", not "leave alone".
pub fn validate_custom_field_values(
    definitions: &[CustomFieldDefinition],
    raw: Option<Value>,
) -> Result<Option<Value>, DomainError> {
    let active: Vec<&CustomFieldDefinition> =
        definitions.iter().filter(|d| d.is_active()).collect();

    let input_object = match raw {
        None => None,
        Some(Value::Null) => None,
        Some(Value::Object(map)) => Some(map),
        Some(_) => {
            return Err(DomainError::InvalidCustomFieldValue(
                "custom field values must be a JSON object".to_string(),
            ));
        }
    };

    // Nothing submitted: only valid if no active field is required.
    let Some(input_object) = input_object else {
        if let Some(missing) = active.iter().find(|d| d.is_required()) {
            return Err(DomainError::InvalidCustomFieldValue(format!(
                "'{}' is required",
                any_label(missing)
            )));
        }
        return Ok(None);
    };

    // Allow-list: reject any key that isn't a known, active field.
    for key in input_object.keys() {
        if !active.iter().any(|d| d.field_key() == key) {
            return Err(DomainError::InvalidCustomFieldValue(format!(
                "unknown custom field '{key}'"
            )));
        }
    }

    let mut output = Map::new();
    for definition in &active {
        let raw_value = input_object.get(definition.field_key());

        let is_blank = matches!(raw_value, None | Some(Value::Null))
            || matches!(raw_value, Some(Value::String(s)) if s.trim().is_empty());

        if is_blank {
            if definition.is_required() {
                return Err(DomainError::InvalidCustomFieldValue(format!(
                    "'{}' is required",
                    any_label(definition)
                )));
            }
            continue;
        }

        let value = raw_value.expect("checked non-blank above");
        let validated = validate_one(definition, value)?;
        output.insert(definition.field_key().to_string(), validated);
    }

    if output.is_empty() {
        return Ok(None);
    }

    let result = Value::Object(output);
    let size = serde_json::to_vec(&result)
        .map_err(|e| DomainError::InvalidCustomFieldValue(format!("failed to serialize: {e}")))?
        .len();
    if size > MAX_TOTAL_PAYLOAD_BYTES {
        return Err(DomainError::InvalidCustomFieldValue(
            "custom field values exceed the maximum allowed size".to_string(),
        ));
    }

    Ok(Some(result))
}

fn validate_one(definition: &CustomFieldDefinition, value: &Value) -> Result<Value, DomainError> {
    use crate::domain::entities::custom_field_definition::CustomFieldType;

    match definition.field_type() {
        CustomFieldType::Text => {
            let Value::String(s) = value else {
                return Err(type_error(definition, "a string"));
            };
            if s.chars().any(|c| c.is_control()) {
                return Err(DomainError::InvalidCustomFieldValue(format!(
                    "'{}' contains invalid control characters",
                    any_label(definition)
                )));
            }
            if s.chars().count() > MAX_TEXT_VALUE_LEN {
                return Err(DomainError::InvalidCustomFieldValue(format!(
                    "'{}' exceeds the maximum length of {MAX_TEXT_VALUE_LEN} characters",
                    any_label(definition)
                )));
            }
            Ok(Value::String(s.clone()))
        }
        CustomFieldType::Number => {
            let Value::Number(n) = value else {
                return Err(type_error(definition, "a number"));
            };
            if let Some(f) = n.as_f64() {
                if !f.is_finite() {
                    return Err(DomainError::InvalidCustomFieldValue(format!(
                        "'{}' must be a finite number",
                        any_label(definition)
                    )));
                }
            }
            Ok(Value::Number(n.clone()))
        }
        CustomFieldType::Boolean => {
            let Value::Bool(b) = value else {
                return Err(type_error(definition, "a boolean"));
            };
            Ok(Value::Bool(*b))
        }
        CustomFieldType::Date => {
            let Value::String(s) = value else {
                return Err(type_error(definition, "a date string (YYYY-MM-DD)"));
            };
            if NaiveDate::parse_from_str(s, "%Y-%m-%d").is_err() {
                return Err(DomainError::InvalidCustomFieldValue(format!(
                    "'{}' must be a valid date in YYYY-MM-DD format",
                    any_label(definition)
                )));
            }
            Ok(Value::String(s.clone()))
        }
        CustomFieldType::Select => {
            let Value::String(s) = value else {
                return Err(type_error(definition, "a string"));
            };
            // Each option is a per-language map; the submitted value is
            // whatever text the sign-up page resolved into the <select> for
            // whichever locale it was rendered in, which this validator has
            // no way to know — so a match in ANY language's translation of
            // ANY choice is accepted, not just one fixed locale.
            let options = definition.options().unwrap_or(&[]);
            if !options
                .iter()
                .any(|entry| entry.values().any(|opt| opt == s))
            {
                return Err(DomainError::InvalidCustomFieldValue(format!(
                    "'{}' must be one of the allowed options",
                    any_label(definition)
                )));
            }
            Ok(Value::String(s.clone()))
        }
    }
}

fn type_error(definition: &CustomFieldDefinition, expected: &str) -> DomainError {
    DomainError::InvalidCustomFieldValue(format!("'{}' must be {expected}", any_label(definition)))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn en(value: &str) -> HashMap<String, String> {
        HashMap::from([("en".to_string(), value.to_string())])
    }

    fn field(
        field_key: &str,
        field_type: crate::domain::entities::custom_field_definition::CustomFieldType,
        options: Option<Vec<&str>>,
        is_required: bool,
    ) -> CustomFieldDefinition {
        CustomFieldDefinition::new(
            Uuid::now_v7(),
            field_key.to_string(),
            en(field_key),
            None,
            field_type,
            options.map(|opts| opts.into_iter().map(en).collect()),
            is_required,
            0,
        )
    }

    use crate::domain::entities::custom_field_definition::CustomFieldType;

    #[test]
    fn happy_path_one_of_each_type() {
        let defs = vec![
            field("nickname", CustomFieldType::Text, None, false),
            field("age", CustomFieldType::Number, None, false),
            field("newsletter", CustomFieldType::Boolean, None, false),
            field("birthday", CustomFieldType::Date, None, false),
            field("size", CustomFieldType::Select, Some(vec!["s", "m"]), false),
        ];
        let raw = json!({
            "nickname": "gabs",
            "age": 30,
            "newsletter": true,
            "birthday": "1990-01-01",
            "size": "m",
        });
        let result = validate_custom_field_values(&defs, Some(raw))
            .unwrap()
            .unwrap();
        assert_eq!(result["nickname"], "gabs");
        assert_eq!(result["age"], 30);
        assert_eq!(result["newsletter"], true);
        assert_eq!(result["size"], "m");
    }

    #[test]
    fn required_field_missing_is_rejected() {
        let defs = vec![field("nickname", CustomFieldType::Text, None, true)];
        let err = validate_custom_field_values(&defs, None).unwrap_err();
        assert!(matches!(err, DomainError::InvalidCustomFieldValue(_)));
    }

    #[test]
    fn unknown_key_is_rejected() {
        let defs = vec![field("nickname", CustomFieldType::Text, None, false)];
        let raw = json!({"nickname": "gabs", "sneaky": "value"});
        let err = validate_custom_field_values(&defs, Some(raw)).unwrap_err();
        assert!(matches!(err, DomainError::InvalidCustomFieldValue(_)));
    }

    #[test]
    fn oversized_text_is_rejected() {
        let defs = vec![field("bio", CustomFieldType::Text, None, false)];
        let raw = json!({"bio": "a".repeat(MAX_TEXT_VALUE_LEN + 1)});
        let err = validate_custom_field_values(&defs, Some(raw)).unwrap_err();
        assert!(matches!(err, DomainError::InvalidCustomFieldValue(_)));
    }

    #[test]
    fn select_value_not_in_options_is_rejected() {
        let defs = vec![field(
            "size",
            CustomFieldType::Select,
            Some(vec!["s", "m"]),
            false,
        )];
        let raw = json!({"size": "<script>alert(1)</script>"});
        let err = validate_custom_field_values(&defs, Some(raw)).unwrap_err();
        assert!(matches!(err, DomainError::InvalidCustomFieldValue(_)));
    }

    #[test]
    fn control_characters_in_text_are_rejected() {
        let defs = vec![field("bio", CustomFieldType::Text, None, false)];
        let raw = json!({"bio": "hello\u{0000}world"});
        let err = validate_custom_field_values(&defs, Some(raw)).unwrap_err();
        assert!(matches!(err, DomainError::InvalidCustomFieldValue(_)));
    }

    #[test]
    fn malformed_date_is_rejected() {
        let defs = vec![field("birthday", CustomFieldType::Date, None, false)];
        let raw = json!({"birthday": "not-a-date"});
        let err = validate_custom_field_values(&defs, Some(raw)).unwrap_err();
        assert!(matches!(err, DomainError::InvalidCustomFieldValue(_)));
    }

    #[test]
    fn total_payload_size_cap_is_enforced() {
        let defs = vec![field("bio", CustomFieldType::Text, None, false)];
        // A single field is capped at 1000 chars, well under 10KB, so exercise the
        // aggregate cap with many fields instead.
        let mut defs_many = Vec::new();
        let mut raw_map = Map::new();
        for i in 0..20 {
            let key = format!("field_{i}");
            defs_many.push(field(&key, CustomFieldType::Text, None, false));
            raw_map.insert(key, json!("x".repeat(1000)));
        }
        let _ = &defs; // keep single-field fixture referenced for clarity
        let err =
            validate_custom_field_values(&defs_many, Some(Value::Object(raw_map))).unwrap_err();
        assert!(matches!(err, DomainError::InvalidCustomFieldValue(_)));
    }

    #[test]
    fn inactive_fields_are_ignored_entirely() {
        let mut inactive = field("legacy", CustomFieldType::Text, None, true);
        inactive.update(
            en("legacy"),
            None,
            None,
            true,
            0,
            false, // is_active = false
        );
        let defs = vec![inactive];
        // Even though 'legacy' is required, it's inactive so it's excluded from
        // enforcement — and any value submitted for it is rejected as unknown.
        let result = validate_custom_field_values(&defs, None).unwrap();
        assert_eq!(result, None);
    }
}
