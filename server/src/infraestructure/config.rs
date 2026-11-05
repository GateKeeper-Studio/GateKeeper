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

// Every setting the server reads, resolved and validated ONCE at startup.
//
// Nothing outside this module calls `std::env::var`: the composition root
// (`crate::bootstrap`) and the background tasks receive this struct. One place to
// read means one place to validate, and a test can build a config without touching
// the process environment.
#[derive(Clone)]
pub struct AppConfig {
    pub app_env: AppEnv,
    pub database_url: String,
    /// Optional separate connection string used ONLY to run migrations, so the
    /// runtime role can be a DML-only user (see docs/ARCHITECTURE-REVIEW.md S8).
    /// Falls back to `database_url`.
    pub database_migration_url: Option<String>,
    /// Run pending migrations at startup. Turn off when a separate job (with the
    /// migration role) owns schema changes.
    pub run_migrations: bool,
    pub database: DatabasePoolConfig,
    pub port: u16,
    /// Separate listener for `/metrics`. `None` disables it. The main API port no
    /// longer serves metrics, so a public API does not expose the registry.
    pub metrics_port: Option<u16>,
    pub log_json: bool,

    /// Public issuer URL pinned as `iss` in every token.
    pub oauth_issuer: String,
    /// Base URL of the hosted login UI (reset links, WebAuthn origin default).
    pub auth_ui_base_url: String,
    pub webauthn: WebauthnConfig,
    pub geoip: Option<GeoIpConfig>,
    /// (document_type, current_version) pairs whose acceptance is required.
    pub required_policy_versions: Vec<(String, String)>,
    /// Maximum Argon2id hashes in flight (64 MiB each). Default: one per CPU core.
    pub password_hash_concurrency: usize,
    /// Grace window after a refresh-token rotation in which presenting the old token
    /// again is a concurrent refresh (reissue), not reuse (revoke the family).
    pub refresh_reuse_leeway_seconds: i64,
    /// How long published signing keys stay cached in process.
    pub signing_key_cache_ttl_seconds: u64,

    /// HS256 key for platform-admin access tokens. Independent of the password
    /// pepper so it can be rotated without invalidating admin password hashes.
    pub admin_token_signing_key: String,
    /// Previous admin-token signing key, still accepted for verification during a
    /// rotation (tokens live 15 minutes, so it can be removed shortly after).
    pub admin_token_previous_signing_key: Option<String>,
    /// Argon2 pepper for platform-admin passwords. Changing it invalidates every
    /// admin password hash, so it is NOT rotated with the signing key.
    pub admin_password_pepper: String,

    /// Allowed CORS origins. Empty means "permissive" (any origin) — intended for
    /// local development only. Set CORS_ALLOWED_ORIGINS (comma-separated) in any
    /// shared/production environment to restrict it.
    pub cors_allowed_origins: Vec<String>,
    /// Reverse proxies whose `X-Forwarded-For` / `X-Real-IP` are believed (CIDR
    /// blocks or single addresses). Empty means the TCP peer is always the client.
    pub trusted_proxies: Vec<String>,

    /// 32-byte AES-256-GCM key sealing every secret at rest (SMTP passwords,
    /// webhook secrets, TOTP seeds, tenant signing keys, tenant password peppers).
    /// Sourced from SECRETS_ENCRYPTION_KEY (legacy name: EMAIL_ENCRYPTION_KEY).
    pub secrets_encryption_key: [u8; 32],
    /// Retired keys, used only to decrypt values sealed before a rotation.
    pub secrets_previous_keys: Vec<[u8; 32]>,

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

// Connection-pool sizing. The default of 10 is conservative; a deployment serving
// real traffic should raise it toward what Postgres itself allows, divided by the
// number of replicas.
#[derive(Debug, Clone, Copy)]
pub struct DatabasePoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
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
#[derive(Debug, Clone)]
pub struct WebauthnConfig {
    pub rp_id: String,
    pub origin: String,
    pub additional_origins: Vec<String>,
}

// MaxMind GeoLite2/GeoIP2 database paths for IP enrichment. Optional: when
// GEOIP_CITY_DB_PATH is unset the composition root wires a no-op resolver, so geo
// rules (new country, impossible travel) simply never fire and the login flow keeps
// working.
#[derive(Debug, Clone)]
pub struct GeoIpConfig {
    pub city_db_path: String,
    pub asn_db_path: Option<String>,
}

const DEFAULT_ISSUER: &str = "http://localhost:8080";
const DEFAULT_AUTH_UI_BASE_URL: &str = "http://localhost:5173";
const DEFAULT_METRICS_PORT: u16 = 9090;

impl AppConfig {
    /// Loads `.env` (if present), resolves `APP_ENV`, validates the strict-mode
    /// contract and reads every setting. Panics with a list of problems on a
    /// misconfigured strict deployment — refusing to start beats serving traffic on
    /// development defaults.
    pub fn from_env() -> Self {
        load_dotenv();

        let app_env = resolve_app_env();
        // Runs before anything is read so a misconfigured deploy dies at boot with a
        // list of what is missing, rather than serving traffic on dev defaults.
        validate_environment(app_env);

        let auth_ui_base_url =
            env_string("AUTH_UI_BASE_URL").unwrap_or_else(|| DEFAULT_AUTH_UI_BASE_URL.to_string());
        let legacy_jwt_secret = env_string("JWT_SECRET");
        let admin_password_pepper = env_string("ADMIN_PASSWORD_PEPPER")
            .or_else(|| legacy_jwt_secret.clone())
            .expect("ADMIN_PASSWORD_PEPPER (or the legacy JWT_SECRET) must be set");
        let admin_token_signing_key = match env_string("ADMIN_TOKEN_SIGNING_KEY") {
            Some(key) => key,
            None => {
                // Development only: strict mode refuses this (validate_environment).
                tracing::warn!(
                    "ADMIN_TOKEN_SIGNING_KEY unset — signing admin tokens with the password \
                     pepper. Development only; set an independent key."
                );
                admin_password_pepper.clone()
            }
        };

        let secrets_encryption_key = resolve_secrets_encryption_key();
        let secrets_previous_keys = env_list("SECRETS_ENCRYPTION_PREVIOUS_KEYS")
            .iter()
            .map(|raw| {
                parse_key(raw).unwrap_or_else(|| {
                    panic!(
                        "SECRETS_ENCRYPTION_PREVIOUS_KEYS entries must be base64 or hex of \
                         exactly 32 bytes"
                    )
                })
            })
            .collect();

        Self {
            app_env,
            database_url: env_string("DATABASE_URL").expect("DATABASE_URL must be set"),
            database_migration_url: env_string("DATABASE_MIGRATION_URL"),
            run_migrations: env_flag("RUN_MIGRATIONS", true),
            database: DatabasePoolConfig {
                max_connections: env_parsed("DATABASE_MAX_CONNECTIONS", 10),
                min_connections: env_parsed("DATABASE_MIN_CONNECTIONS", 0),
                acquire_timeout_seconds: env_parsed("DATABASE_ACQUIRE_TIMEOUT_SECONDS", 5),
            },

            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a valid number"),
            metrics_port: resolve_metrics_port(),
            log_json: resolve_log_json(app_env),

            oauth_issuer: env_string("OAUTH_ISSUER").unwrap_or_else(|| DEFAULT_ISSUER.to_string()),
            webauthn: resolve_webauthn(&auth_ui_base_url),
            auth_ui_base_url,
            geoip: resolve_geoip_config(),
            required_policy_versions: vec![
                (
                    "terms_of_service".to_string(),
                    env_string("POLICY_TERMS_VERSION").unwrap_or_else(|| "1".to_string()),
                ),
                (
                    "privacy_policy".to_string(),
                    env_string("POLICY_PRIVACY_VERSION").unwrap_or_else(|| "1".to_string()),
                ),
            ],
            password_hash_concurrency: env_parsed(
                "PASSWORD_HASH_CONCURRENCY",
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4),
            ),
            refresh_reuse_leeway_seconds: env_parsed(
                "REFRESH_REUSE_LEEWAY_SECONDS",
                crate::application::use_cases::oauth::token_refresh::use_case::DEFAULT_REFRESH_REUSE_LEEWAY_SECONDS,
            ),
            // This is the window in which a RETIRED key could still be accepted, so it
            // wants to stay short. A newly published key is not bounded by it: the auth
            // middleware invalidates and refetches when a token fails to verify.
            signing_key_cache_ttl_seconds: env_parsed("SIGNING_KEY_CACHE_TTL_SECONDS", 60),

            admin_token_signing_key,
            admin_token_previous_signing_key: env_string("ADMIN_TOKEN_PREVIOUS_SIGNING_KEY"),
            admin_password_pepper,

            cors_allowed_origins: env_list("CORS_ALLOWED_ORIGINS"),
            trusted_proxies: env_list("TRUSTED_PROXIES"),

            secrets_encryption_key,
            secrets_previous_keys,

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

    /// The connection string migrations run with.
    pub fn migration_database_url(&self) -> &str {
        self.database_migration_url
            .as_deref()
            .unwrap_or(&self.database_url)
    }
}

/// Loads `.env` from the working directory if one exists. Idempotent; call it
/// before reading `APP_ENV` so a value set only in `.env` is honoured.
pub fn load_dotenv() {
    dotenvy::dotenv().ok();
}

// JSON logs unless LOG_FORMAT says otherwise; JSON is the default outside
// development so a production deployment is searchable without anyone remembering
// to set it.
pub fn resolve_log_json(app_env: AppEnv) -> bool {
    match env_string("LOG_FORMAT") {
        Some(value) => value.eq_ignore_ascii_case("json"),
        None => !app_env.is_development(),
    }
}

// `METRICS_PORT` unset ⇒ 9090; `off` / `disabled` / `0` ⇒ no metrics listener.
fn resolve_metrics_port() -> Option<u16> {
    match env_string("METRICS_PORT") {
        None => Some(DEFAULT_METRICS_PORT),
        Some(raw) if matches!(raw.to_ascii_lowercase().as_str(), "off" | "disabled" | "0") => None,
        Some(raw) => Some(
            raw.parse()
                .expect("METRICS_PORT must be a port number or `off`"),
        ),
    }
}

// Returns None when GEOIP_CITY_DB_PATH is absent (geo enrichment disabled).
fn resolve_geoip_config() -> Option<GeoIpConfig> {
    let city_db_path = env_string("GEOIP_CITY_DB_PATH")?;
    Some(GeoIpConfig {
        city_db_path,
        asn_db_path: env_string("GEOIP_ASN_DB_PATH"),
    })
}

fn resolve_webauthn(auth_ui_base_url: &str) -> WebauthnConfig {
    let origin = env_string("WEBAUTHN_RP_ORIGIN").unwrap_or_else(|| auth_ui_base_url.to_string());

    let rp_id = env_string("WEBAUTHN_RP_ID").unwrap_or_else(|| {
        // Effective domain of the origin: strip scheme, port and path.
        origin
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split([':', '/'])
            .next()
            .unwrap_or("localhost")
            .to_string()
    });

    let additional_origins = env_list("WEBAUTHN_ADDITIONAL_ORIGINS")
        .into_iter()
        .filter(|o| *o != origin)
        .collect();

    WebauthnConfig {
        rp_id,
        origin,
        additional_origins,
    }
}

// A non-empty, trimmed env var.
fn env_string(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

// A comma-separated env var, trimmed, empties dropped.
fn env_list(key: &str) -> Vec<String> {
    std::env::var(key)
        .map(|raw| {
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
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
// SECRETS_ENCRYPTION_KEY is unset, so a fresh clone runs without ceremony.
//
// It is deliberately a constant rather than something derived from another
// secret: leaking the admin-token key must not also decrypt every stored SMTP
// password, webhook signing secret, TOTP seed, tenant pepper and signing key.
const DEV_SECRETS_ENCRYPTION_KEY: [u8; 32] = *b"gatekeeper-development-key-32byt";

fn parse_key(raw: &str) -> Option<[u8; 32]> {
    let bytes = decode_key_material(raw.trim())?;
    <[u8; 32]>::try_from(bytes.as_slice()).ok()
}

// The current secrets key: SECRETS_ENCRYPTION_KEY, or the legacy name
// EMAIL_ENCRYPTION_KEY (the key never protected only email — the name was
// misleading). Required outside development (enforced up-front by
// `validate_environment`); in development an unset value falls back to a
// well-known constant.
pub fn resolve_secrets_encryption_key() -> [u8; 32] {
    let raw = env_string("SECRETS_ENCRYPTION_KEY").or_else(|| {
        let legacy = env_string("EMAIL_ENCRYPTION_KEY");
        if legacy.is_some() {
            tracing::warn!(
                "EMAIL_ENCRYPTION_KEY is deprecated — rename it to SECRETS_ENCRYPTION_KEY"
            );
        }
        legacy
    });

    if let Some(raw) = raw {
        return parse_key(&raw).unwrap_or_else(|| {
            panic!("SECRETS_ENCRYPTION_KEY must be base64 or hex encoding exactly 32 bytes")
        });
    }

    if !resolve_app_env().is_development() {
        panic!(
            "SECRETS_ENCRYPTION_KEY must be set when APP_ENV is not `development` \
             (generate one with: openssl rand -base64 32)"
        );
    }

    tracing::warn!(
        "SECRETS_ENCRYPTION_KEY unset — using the built-in development key. \
         Stored secrets are NOT confidential. Never run this way outside development."
    );
    DEV_SECRETS_ENCRYPTION_KEY
}

// Minimum length of the admin-token signing key and the admin password pepper
// outside development. A short or guessable signing key is directly forgeable.
const MIN_ADMIN_SECRET_LEN: usize = 32;

/// Fail-fast validation of the deployment environment.
///
/// Every variable checked here has a localhost-friendly fallback in this module. Those fallbacks are correct for development and
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
            "ADMIN_TOKEN_SIGNING_KEY",
            "HS256 key for platform-admin tokens; must be independent of the admin password pepper",
        ),
    ] {
        if missing(key) {
            problems.push(format!("{key} must be set — {why}"));
        }
    }

    if missing("SECRETS_ENCRYPTION_KEY") && missing("EMAIL_ENCRYPTION_KEY") {
        problems.push(
            "SECRETS_ENCRYPTION_KEY must be set — 32-byte AES-256-GCM key protecting stored \
             secrets (the legacy name EMAIL_ENCRYPTION_KEY is still accepted)"
                .to_string(),
        );
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

    // The admin password pepper: ADMIN_PASSWORD_PEPPER, or JWT_SECRET for
    // deployments created before the two roles were split (existing admin hashes
    // were made with it, so it must keep its value).
    let pepper = get("ADMIN_PASSWORD_PEPPER")
        .filter(|v| !v.trim().is_empty())
        .or_else(|| get("JWT_SECRET").filter(|v| !v.trim().is_empty()));
    match pepper.as_deref().map(str::trim) {
        None => problems.push(
            "ADMIN_PASSWORD_PEPPER must be set (or keep the legacy JWT_SECRET) — the Argon2 \
             pepper for platform-admin passwords"
                .to_string(),
        ),
        Some(p) if p.len() < MIN_ADMIN_SECRET_LEN => problems.push(format!(
            "ADMIN_PASSWORD_PEPPER/JWT_SECRET is {} characters — at least {MIN_ADMIN_SECRET_LEN} \
             required (generate one with: openssl rand -base64 48)",
            p.len()
        )),
        Some(_) => {}
    }

    if let Some(key) = get("ADMIN_TOKEN_SIGNING_KEY").map(|v| v.trim().to_string())
        && !key.is_empty()
    {
        if key.len() < MIN_ADMIN_SECRET_LEN {
            problems.push(format!(
                "ADMIN_TOKEN_SIGNING_KEY is {} characters — at least {MIN_ADMIN_SECRET_LEN} \
                 required (generate one with: openssl rand -base64 48)",
                key.len()
            ));
        }
        // One secret in two roles is what made the old JWT_SECRET impossible to
        // rotate: changing it locked every admin out.
        if pepper.as_deref().map(str::trim) == Some(key.as_str()) {
            problems.push(
                "ADMIN_TOKEN_SIGNING_KEY must differ from the admin password pepper \
                 (ADMIN_PASSWORD_PEPPER/JWT_SECRET) so the signing key can be rotated"
                    .to_string(),
            );
        }
    }

    problems
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
                "SECRETS_ENCRYPTION_KEY",
                "aGVsbG8td29ybGQtdGhpcy1pcy0zMi1ieXQ=",
            ),
            ("WEBAUTHN_RP_ORIGIN", "https://login.example.com"),
            ("ADMIN_PASSWORD_PEPPER", "0123456789abcdef0123456789abcdef"),
            (
                "ADMIN_TOKEN_SIGNING_KEY",
                "fedcba9876543210fedcba9876543210",
            ),
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
            "SECRETS_ENCRYPTION_KEY",
            "ADMIN_TOKEN_SIGNING_KEY",
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
    fn a_short_pepper_is_rejected() {
        let mut env = valid_env();
        env.insert("ADMIN_PASSWORD_PEPPER", "my-ultimate-jwt-secret");
        assert!(
            problems_for(&env)
                .iter()
                .any(|p| p.starts_with("ADMIN_PASSWORD_PEPPER"))
        );
    }

    #[test]
    fn the_legacy_jwt_secret_still_serves_as_the_pepper() {
        let mut env = valid_env();
        env.remove("ADMIN_PASSWORD_PEPPER");
        env.insert("JWT_SECRET", "0123456789abcdef0123456789abcdef");
        assert!(problems_for(&env).is_empty());

        env.remove("JWT_SECRET");
        assert!(
            problems_for(&env)
                .iter()
                .any(|p| p.starts_with("ADMIN_PASSWORD_PEPPER"))
        );
    }

    #[test]
    fn the_legacy_encryption_key_name_is_accepted() {
        let mut env = valid_env();
        let key = env.remove("SECRETS_ENCRYPTION_KEY").unwrap();
        env.insert("EMAIL_ENCRYPTION_KEY", key);
        assert!(problems_for(&env).is_empty());
    }

    #[test]
    fn the_signing_key_must_not_reuse_the_pepper() {
        let mut env = valid_env();
        env.insert(
            "ADMIN_TOKEN_SIGNING_KEY",
            "0123456789abcdef0123456789abcdef",
        );
        assert!(problems_for(&env).iter().any(|p| p.contains("must differ")));
    }

    #[test]
    fn a_short_signing_key_is_rejected() {
        let mut env = valid_env();
        env.insert("ADMIN_TOKEN_SIGNING_KEY", "short");
        assert!(
            problems_for(&env)
                .iter()
                .any(|p| p.starts_with("ADMIN_TOKEN_SIGNING_KEY"))
        );
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
