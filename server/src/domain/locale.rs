// The closed set of languages any tenant/client admin can localize
// end-user-facing text into. Kept as a plain domain constant (no framework
// dependency) since "which languages exist" is a business rule, not
// infrastructure — every localized field (login-appearance heading/subtext,
// custom-field label/description/options) validates its locale keys against
// this list.
//
// Must be kept in sync by hand with the dashboard's own locale list
// (`dashboard/project.inlang/settings.json` / `dashboard/src/lib/i18n/locales.ts`)
// — there is no shared source of truth across the Rust/TS boundary.
pub const SUPPORTED_LOCALES: [&str; 3] = ["en", "es", "pt-br"];

pub fn is_supported_locale(code: &str) -> bool {
    SUPPORTED_LOCALES.contains(&code)
}
