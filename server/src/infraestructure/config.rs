/// The deployment environment, sourced from `APP_ENV`.
///
/// `Development` keeps the localhost-friendly fallbacks scattered through this
/// module. Every other value selects `Strict`, which makes those fallbacks a
/// startup panic instead: an identity provider that silently boots with a
/// localhost issuer, a permissive CORS policy, or a key derived from another
/// key is worse than one that refuses to start.
///
/// **Unset defaults to `Strict`.** Defaulting to `Development` would reintroduce
/// exactly the silent-misconfiguration failure this type exists to prevent — a
/// deploy that forgets `APP_ENV` would get every dev fallback. Local development
/// opts in explicitly (`APP_ENV=development`, as shipped in `.env.example`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Strict,
}

impl AppEnv {
    pub fn is_development(self) -> bool {
        matches!(self, AppEnv::Development)
    }
}

pub fn resolve_app_env() -> AppEnv {
    resolve_app_env_from(std::env::var("APP_ENV").ok().as_deref())
}

// Split from the env read so it is unit-testable — see `production_config_problems`.
fn resolve_app_env_from(raw: Option<&str>) -> AppEnv {
    match raw.map(|r| r.trim().to_ascii_lowercase()) {
        Some(value) => match value.as_str() {
            "development" | "dev" | "local" | "test" => AppEnv::Development,
            _ => AppEnv::Strict,
        },
        None => AppEnv::Strict,
    }
}

pub struct AppConfig {
    pub app_env: AppEnv,
    pub database_url: String,
    pub port: u16,
    pub jwt_secret: String,
    /// Allowed CORS origins. Empty means "permissive" (any origin) — intended for
    /// local development only. Set CORS_ALLOWED_ORIGINS (comma-separated) in any
    /// shared/production environment to restrict it.
    pub cors_allowed_origins: Vec<String>,
    /// 32-byte AES-256-GCM master key used to encrypt stored SMTP provider
    /// passwords. Sourced from EMAIL_ENCRYPTION_KEY (base64 or hex of 32 bytes).
    /// Required outside development; in development an unset value falls back to
    /// a fixed dev constant that is deliberately NOT derived from JWT_SECRET.
    pub email_encryption_key: [u8; 32],

    // ── Outbound webhooks ────────────────────────────────────────────────────
    /// Master switch for the background dispatcher. When false the outbox still
    /// fills (events are never lost), nothing is just delivered — useful for a
    /// replica that should serve traffic without also draining the queue.
    pub webhooks_enabled: bool,
    /// How often the dispatcher drains the outbox.
    pub webhook_poll_interval_seconds: u64,
    /// Rows claimed per phase per tick.
    pub webhook_batch_size: i64,
    /// Per-request timeout. A consumer that stalls must not pin a dispatcher slot.
    pub webhook_timeout_seconds: u64,
    /// Master switch for the background retention purge. Off means expired rows
    /// accumulate until something else deletes them — only turn it off if an
    /// external job owns retention instead.
    pub retention_purge_enabled: bool,
    /// How often the purge runs. Hourly by default: the rows it deletes are already
    /// expired and unusable, so promptness matters far less than not hammering the
    /// database.
    pub retention_purge_interval_seconds: u64,
    /// SECURITY: relaxes the SSRF guard on subscription URLs so they may target
    /// private/loopback addresses, and permits plain http on loopback. Intended for
    /// local development and self-hosted single-network deployments ONLY — leaving
    /// it on in a cloud environment means a tenant admin can point a webhook at the
    /// instance metadata endpoint and read your IAM credentials. Defaults to false.
    pub webhook_allow_private_targets: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let app_env = resolve_app_env();
        // Runs before anything is read so a misconfigured deploy dies at boot with a
        // list of what is missing, rather than serving traffic on dev defaults.
        validate_environment(app_env);

        Self {
            app_env,
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),

            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a valid number"),

            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),

            cors_allowed_origins: std::env::var("CORS_ALLOWED_ORIGINS")
                .map(|raw| {
                    raw.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),

            email_encryption_key: resolve_email_encryption_key(),

            retention_purge_enabled: env_flag("RETENTION_PURGE_ENABLED", true),
            retention_purge_interval_seconds: env_parsed("RETENTION_PURGE_INTERVAL_SECONDS", 3600),

            webhooks_enabled: env_flag("WEBHOOKS_ENABLED", true),
            webhook_poll_interval_seconds: env_parsed("WEBHOOK_POLL_INTERVAL_SECONDS", 5),
            webhook_batch_size: env_parsed("WEBHOOK_BATCH_SIZE", 100),
            webhook_timeout_seconds: env_parsed("WEBHOOK_TIMEOUT_SECONDS", 10),
            // Defaults to false: the safe posture is refusing private targets, and an
            // operator who needs the relaxation has to ask for it explicitly.
            webhook_allow_private_targets: env_flag("WEBHOOK_ALLOW_PRIVATE_TARGETS", false),
        }
    }
}

// `create_router` builds the composition root from a pool alone (it does not take
// AppConfig), so these are exposed as standalone resolvers — the same shape as
// `resolve_email_encryption_key`.
pub fn resolve_webhook_allow_private_targets() -> bool {
    env_flag("WEBHOOK_ALLOW_PRIVATE_TARGETS", false)
}

pub fn resolve_webhook_timeout_seconds() -> u64 {
    env_parsed("WEBHOOK_TIMEOUT_SECONDS", 10)
}

// How long a tenant's published signing keys stay cached in process.
//
// This is the window in which a RETIRED key could still be accepted, so it wants to
// stay short. The opposite case — a newly published key not yet cached — is not
// bounded by this at all: the auth middleware invalidates and refetches when a token
// fails to verify, so rotation takes effect immediately.
pub fn resolve_signing_key_cache_ttl_seconds() -> u64 {
    env_parsed("SIGNING_KEY_CACHE_TTL_SECONDS", 60)
}

// Connection-pool sizing. The default of 10 is conservative; a deployment serving
// real traffic should raise it toward what Postgres itself allows, divided by the
// number of replicas. Exposed as env because the right number depends entirely on
// the deployment, not on the code.
pub fn resolve_db_max_connections() -> u32 {
    env_parsed("DATABASE_MAX_CONNECTIONS", 10)
}

pub fn resolve_db_min_connections() -> u32 {
    env_parsed("DATABASE_MIN_CONNECTIONS", 0)
}

pub fn resolve_db_acquire_timeout_seconds() -> u64 {
    env_parsed("DATABASE_ACQUIRE_TIMEOUT_SECONDS", 5)
}

// A boolean env var. Anything other than an explicit "false"/"0" reads as true when
// the default is true, so a typo cannot silently disable delivery.
fn env_flag(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(raw) => !matches!(raw.trim().to_ascii_lowercase().as_str(), "false" | "0" | ""),
        Err(_) => default,
    }
}

// A numeric env var, falling back to the default when unset or unparseable rather
// than panicking — a bad tuning value should not stop the service booting.
fn env_parsed<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse().ok())
        .unwrap_or(default)
}

// A fixed, publicly-known key used ONLY when APP_ENV selects development and
// EMAIL_ENCRYPTION_KEY is unset, so a fresh clone runs without ceremony.
//
// It is deliberately a constant rather than something derived from JWT_SECRET.
// The previous derivation (SHA-256 of JWT_SECRET) collapsed three separate
// trust domains into one variable: leaking JWT_SECRET meant an attacker could
// forge platform-admin tokens, crack admin passwords offline (it is also the
// admin Argon2 pepper), AND decrypt every stored SMTP password, webhook signing
// secret, TOTP seed and tenant signing key. Those key roles are now independent
// and can be rotated separately.
const DEV_EMAIL_ENCRYPTION_KEY: [u8; 32] = *b"gatekeeper-development-key-32byt";

// EMAIL_ENCRYPTION_KEY must decode (base64 or hex) to exactly 32 bytes. Required
// outside development (enforced up-front by `validate_environment`); in
// development an unset value falls back to a well-known constant.
pub fn resolve_email_encryption_key() -> [u8; 32] {
    if let Ok(raw) = std::env::var("EMAIL_ENCRYPTION_KEY")
        && !raw.trim().is_empty()
    {
        return match decode_key_material(raw.trim()) {
            Some(bytes) if bytes.len() == 32 => {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                key
            }
            _ => panic!("EMAIL_ENCRYPTION_KEY must be base64 or hex encoding exactly 32 bytes"),
        };
    }

    if !resolve_app_env().is_development() {
        panic!(
            "EMAIL_ENCRYPTION_KEY must be set when APP_ENV is not `development` \
             (generate one with: openssl rand -base64 32)"
        );
    }

    tracing::warn!(
        "EMAIL_ENCRYPTION_KEY unset — using the built-in development key. \
         Stored secrets are NOT confidential. Never run this way outside development."
    );
    DEV_EMAIL_ENCRYPTION_KEY
}

// Minimum JWT_SECRET length outside development. It is the HS256 signing key for
// platform-admin tokens and the Argon2 pepper for their passwords, so a short or
// guessable value is directly forgeable.
const MIN_JWT_SECRET_LEN: usize = 32;

/// Fail-fast validation of the deployment environment.
///
/// Every variable checked here has a localhost-friendly fallback somewhere in
/// this module or in `routes.rs`. Those fallbacks are correct for development and
/// dangerous in production — a wrong `OAUTH_ISSUER` puts a bogus `iss` in every
/// token issued and breaks discovery for every relying party, with no error
/// anywhere. Collecting the checks in one place (rather than at each read site)
/// keeps the production contract readable and reports every problem at once
/// instead of one panic per boot.
///
/// No-op in development.
pub fn validate_environment(app_env: AppEnv) {
    if app_env.is_development() {
        tracing::warn!(
            "APP_ENV selects development — localhost fallbacks are active for \
             OAUTH_ISSUER, CORS, WebAuthn and secret encryption."
        );
        return;
    }

    let problems = production_config_problems(&|key| std::env::var(key).ok());

    // Relaxing the webhook SSRF guard in a cloud environment lets a tenant admin
    // point a subscription at the instance metadata endpoint. Warn rather than
    // refuse: it is legitimate for self-hosted single-network deployments.
    if env_flag("WEBHOOK_ALLOW_PRIVATE_TARGETS", false) {
        tracing::warn!(
            "WEBHOOK_ALLOW_PRIVATE_TARGETS is enabled outside development — webhook URLs may \
             target private/loopback addresses. Safe only on a trusted single network."
        );
    }

    if !problems.is_empty() {
        panic!(
            "invalid configuration for APP_ENV={} ({} problem(s)):\n  - {}\n\nSet APP_ENV=development to use localhost defaults.",
            std::env::var("APP_ENV").unwrap_or_else(|_| "<unset>".to_string()),
            problems.len(),
            problems.join("\n  - "),
        );
    }
}

/// The strict-environment policy, expressed over an arbitrary variable lookup.
///
/// Kept separate from the `std::env` read so it can be unit-tested: `set_var` is
/// `unsafe` in edition 2024 and this crate is `#![forbid(unsafe_code)]`, so tests
/// cannot mutate the real environment (nor would that be sound with parallel
/// tests sharing one process).
fn production_config_problems(get: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    // Absent, empty, or whitespace-only all count as missing.
    let missing = |key: &str| get(key).map(|v| v.trim().is_empty()).unwrap_or(true);

    let mut problems: Vec<String> = Vec::new();

    // Present-and-non-empty checks. An empty value is the same failure as an
    // absent one: `CORS_ALLOWED_ORIGINS=""` parses to an empty allow-list, which
    // `build_cors` treats as "allow any origin".
    for (key, why) in [
        (
            "OAUTH_ISSUER",
            "the public issuer URL pinned as `iss` in every token; defaults to http://localhost:8080",
        ),
        (
            "CORS_ALLOWED_ORIGINS",
            "comma-separated browser origins; empty means ANY origin is allowed",
        ),
        (
            "AUTH_UI_BASE_URL",
            "base URL of the hosted login UI; password-reset emails link to it",
        ),
        (
            "EMAIL_ENCRYPTION_KEY",
            "32-byte AES-256-GCM key protecting stored secrets; must be independent of JWT_SECRET",
        ),
    ] {
        if missing(key) {
            problems.push(format!("{key} must be set — {why}"));
        }
    }

    // WebAuthn needs an explicit origin; it otherwise inherits AUTH_UI_BASE_URL and
    // ultimately http://localhost:5173, which silently breaks every passkey ceremony.
    if missing("WEBAUTHN_RP_ORIGIN") && missing("AUTH_UI_BASE_URL") {
        problems.push(
            "WEBAUTHN_RP_ORIGIN (or AUTH_UI_BASE_URL) must be set — the WebAuthn relying-party \
             origin must match the host the browser shows during the ceremony"
                .to_string(),
        );
    }

    // The issuer is the one value where a *present* mistake is as damaging as an
    // absent one: pointing relying parties at localhost is never intended in a
    // non-development deploy.
    if let Some(issuer) = get("OAUTH_ISSUER") {
        let issuer = issuer.trim();
        if issuer.contains("localhost") || issuer.contains("127.0.0.1") {
            problems.push(format!(
                "OAUTH_ISSUER is {issuer:?} — a loopback issuer is not reachable by relying parties"
            ));
        }
    }

    if let Some(secret) = get("JWT_SECRET")
        && secret.trim().len() < MIN_JWT_SECRET_LEN
    {
        problems.push(format!(
            "JWT_SECRET is {} characters — at least {MIN_JWT_SECRET_LEN} required \
             (generate one with: openssl rand -base64 48)",
            secret.trim().len()
        ));
    }

    // `REQUIRE_ADMIN_AUTH=false` degrades the entire admin guard to best-effort:
    // tokens are parsed for audit attribution but a missing or non-admin token no
    // longer blocks the request. It exists as a migration aid only. Mirrors
    // `env_flag`'s parsing so the check and the runtime read agree.
    let admin_auth_enforced = match get("REQUIRE_ADMIN_AUTH") {
        Some(raw) => !matches!(raw.trim().to_ascii_lowercase().as_str(), "false" | "0" | ""),
        None => true,
    };
    if !admin_auth_enforced {
        problems.push(
            "REQUIRE_ADMIN_AUTH=false disables enforcement of the admin guard and is refused \
             outside development"
                .to_string(),
        );
    }

    problems
}

// MaxMind GeoLite2/GeoIP2 database paths for IP enrichment. Optional: when
// GEOIP_CITY_DB_PATH is unset the composition root wires a no-op resolver, so geo
// rules (new country, impossible travel) simply never fire and the login flow keeps
// working. Download GeoLite2-City.mmdb (and optionally GeoLite2-ASN.mmdb) from
// MaxMind and point these at them to enable enrichment.
#[derive(Debug, Clone)]
pub struct GeoIpConfig {
    pub city_db_path: String,
    pub asn_db_path: Option<String>,
}

// Returns None when GEOIP_CITY_DB_PATH is absent (geo enrichment disabled).
pub fn resolve_geoip_config() -> Option<GeoIpConfig> {
    let city_db_path = std::env::var("GEOIP_CITY_DB_PATH").ok()?;
    if city_db_path.trim().is_empty() {
        return None;
    }
    Some(GeoIpConfig {
        city_db_path,
        asn_db_path: std::env::var("GEOIP_ASN_DB_PATH")
            .ok()
            .filter(|p| !p.trim().is_empty()),
    })
}

// WebAuthn relying party identity. The rp_id/origin MUST match the host the
// browser shows while running the ceremony (the auth UI), or registrations and
// assertions fail client-side. Defaults derive from AUTH_UI_BASE_URL so local
// dev works out of the box; set WEBAUTHN_RP_ID / WEBAUTHN_RP_ORIGIN explicitly
// when the auth UI moves to its own domain.
//
// Ceremonies can also run from other first-party apps (e.g. a BFF client's
// /account/security page enrolling a passkey). Every such browser origin must
// be allow-listed via WEBAUTHN_ADDITIONAL_ORIGINS (comma-separated), and all
// origins must share the rp_id's effective domain.
//
// Returns (rp_id, rp_origin, additional_origins).
pub fn resolve_webauthn_rp() -> (String, String, Vec<String>) {
    let origin = std::env::var("WEBAUTHN_RP_ORIGIN")
        .or_else(|_| std::env::var("AUTH_UI_BASE_URL"))
        .unwrap_or_else(|_| "http://localhost:5173".to_string());

    let rp_id = std::env::var("WEBAUTHN_RP_ID").unwrap_or_else(|_| {
        // Effective domain of the origin: strip scheme, port and path.
        origin
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split([':', '/'])
            .next()
            .unwrap_or("localhost")
            .to_string()
    });

    let additional_origins = std::env::var("WEBAUTHN_ADDITIONAL_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|o| !o.is_empty() && *o != origin)
        .map(str::to_string)
        .collect();

    (rp_id, origin, additional_origins)
}

// The legal documents whose acceptance is required, and the version currently in
// force (WI-04). A version bump forces re-acceptance. Sourced from env so it can be
// rolled without a deploy; defaults to "1" for local dev.
//
// Returns (document_type, current_version) pairs.
pub fn resolve_required_policy_versions() -> Vec<(String, String)> {
    let tos = std::env::var("POLICY_TERMS_VERSION").unwrap_or_else(|_| "1".to_string());
    let privacy = std::env::var("POLICY_PRIVACY_VERSION").unwrap_or_else(|_| "1".to_string());
    vec![
        ("terms_of_service".to_string(), tos),
        ("privacy_policy".to_string(), privacy),
    ]
}

// Accepts either base64 (standard) or hex; returns the raw bytes if either decodes.
fn decode_key_material(value: &str) -> Option<Vec<u8>> {
    use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

    if let Ok(bytes) = BASE64.decode(value) {
        return Some(bytes);
    }
    hex::decode(value).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // A fully-valid strict-environment configuration; tests remove or corrupt one
    // entry at a time so each assertion pins exactly one rule.
    fn valid_env() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("OAUTH_ISSUER", "https://id.example.com"),
            ("CORS_ALLOWED_ORIGINS", "https://dash.example.com"),
            ("AUTH_UI_BASE_URL", "https://login.example.com"),
            (
                "EMAIL_ENCRYPTION_KEY",
                "aGVsbG8td29ybGQtdGhpcy1pcy0zMi1ieXQ=",
            ),
            ("WEBAUTHN_RP_ORIGIN", "https://login.example.com"),
            ("JWT_SECRET", "0123456789abcdef0123456789abcdef"),
            ("REQUIRE_ADMIN_AUTH", "true"),
        ])
    }

    fn problems_for(env: &HashMap<&'static str, &'static str>) -> Vec<String> {
        production_config_problems(&|key| env.get(key).map(|v| v.to_string()))
    }

    #[test]
    fn a_fully_configured_environment_has_no_problems() {
        assert!(problems_for(&valid_env()).is_empty());
    }

    #[test]
    fn each_required_variable_is_reported_when_absent() {
        for key in [
            "OAUTH_ISSUER",
            "CORS_ALLOWED_ORIGINS",
            "AUTH_UI_BASE_URL",
            "EMAIL_ENCRYPTION_KEY",
        ] {
            let mut env = valid_env();
            env.remove(key);
            let problems = problems_for(&env);
            assert!(
                problems.iter().any(|p| p.starts_with(key)),
                "removing {key} should be reported, got {problems:?}"
            );
        }
    }

    #[test]
    fn an_empty_value_counts_as_missing() {
        // The specific bug this guards: CORS_ALLOWED_ORIGINS="" parses to an empty
        // allow-list, which build_cors turns into allow_origin(Any).
        let mut env = valid_env();
        env.insert("CORS_ALLOWED_ORIGINS", "   ");
        assert!(
            problems_for(&env)
                .iter()
                .any(|p| p.starts_with("CORS_ALLOWED_ORIGINS"))
        );
    }

    #[test]
    fn a_loopback_issuer_is_rejected_even_though_it_is_set() {
        for issuer in ["http://localhost:8080", "http://127.0.0.1:8080"] {
            let mut env = valid_env();
            env.insert("OAUTH_ISSUER", issuer);
            assert!(
                problems_for(&env).iter().any(|p| p.contains("loopback")),
                "{issuer} should be rejected"
            );
        }
    }

    #[test]
    fn a_short_jwt_secret_is_rejected() {
        let mut env = valid_env();
        env.insert("JWT_SECRET", "my-ultimate-jwt-secret");
        assert!(
            problems_for(&env)
                .iter()
                .any(|p| p.starts_with("JWT_SECRET"))
        );
    }

    #[test]
    fn disabling_the_admin_guard_is_refused() {
        for raw in ["false", "0", ""] {
            let mut env = valid_env();
            env.insert("REQUIRE_ADMIN_AUTH", raw);
            assert!(
                problems_for(&env)
                    .iter()
                    .any(|p| p.starts_with("REQUIRE_ADMIN_AUTH")),
                "REQUIRE_ADMIN_AUTH={raw:?} should be refused"
            );
        }
    }

    #[test]
    fn webauthn_origin_may_be_inherited_from_the_auth_ui_base_url() {
        let mut env = valid_env();
        env.remove("WEBAUTHN_RP_ORIGIN");
        assert!(
            !problems_for(&env).iter().any(|p| p.contains("WEBAUTHN")),
            "AUTH_UI_BASE_URL should satisfy the WebAuthn origin requirement"
        );
    }

    #[test]
    fn app_env_defaults_to_strict_so_a_forgotten_variable_cannot_silently_pass() {
        assert_eq!(resolve_app_env_from(None), AppEnv::Strict);
        assert_eq!(resolve_app_env_from(Some("production")), AppEnv::Strict);
        assert_eq!(resolve_app_env_from(Some("staging")), AppEnv::Strict);
        assert_eq!(
            resolve_app_env_from(Some("development")),
            AppEnv::Development
        );
        assert_eq!(resolve_app_env_from(Some("  DEV  ")), AppEnv::Development);
    }
}
