use std::collections::HashMap;

// Resolves a per-language text map down to a single value for the locale a
// hosted page is actually rendering in: exact locale match, else the
// tenant/client's own default locale, else whatever translation happens to
// exist (so a definition is never blank just because neither the requested
// nor the default locale was filled in). Shared by the OAuth client's
// login-appearance heading/subtext and the tenant's custom-field
// label/description/options resolution.
pub fn resolve_localized<'a>(
    map: &'a HashMap<String, String>,
    locale: &str,
    default_locale: &str,
) -> Option<&'a str> {
    map.get(locale)
        .or_else(|| map.get(default_locale))
        .or_else(|| map.values().next())
        .map(String::as_str)
}
