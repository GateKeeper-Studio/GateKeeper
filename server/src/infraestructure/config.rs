pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
    pub jwt_secret: String,
    /// Allowed CORS origins. Empty means "permissive" (any origin) — intended for
    /// local development only. Set CORS_ALLOWED_ORIGINS (comma-separated) in any
    /// shared/production environment to restrict it.
    pub cors_allowed_origins: Vec<String>,
    /// 32-byte AES-256-GCM master key used to encrypt stored SMTP provider
    /// passwords. Sourced from EMAIL_ENCRYPTION_KEY (base64 or hex of 32 bytes);
    /// if unset, derived from JWT_SECRET via SHA-256 (dev convenience).
    pub email_encryption_key: [u8; 32],
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
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
        }
    }
}

// EMAIL_ENCRYPTION_KEY must decode (base64 or hex) to exactly 32 bytes. When it
// is absent we derive a stable 32-byte key from JWT_SECRET via SHA-256 so local
// dev works out of the box; production should set an independent key.
pub fn resolve_email_encryption_key() -> [u8; 32] {
    use sha2::{Digest, Sha256};

    if let Ok(raw) = std::env::var("EMAIL_ENCRYPTION_KEY") {
        let decoded = decode_key_material(raw.trim());
        match decoded {
            Some(bytes) if bytes.len() == 32 => {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return key;
            }
            _ => panic!("EMAIL_ENCRYPTION_KEY must be base64 or hex encoding exactly 32 bytes"),
        }
    }

    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let digest = Sha256::digest(jwt_secret.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
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
