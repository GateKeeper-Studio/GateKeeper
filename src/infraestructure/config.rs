pub struct AppConfig {
  pub database_url: String,
  pub port: u16,
  pub jwt_secret: String,
  /// Allowed CORS origins. Empty means "permissive" (any origin) — intended for
  /// local development only. Set CORS_ALLOWED_ORIGINS (comma-separated) in any
  /// shared/production environment to restrict it.
  pub cors_allowed_origins: Vec<String>,
}

impl AppConfig {
  pub fn from_env() -> Self {
    dotenvy::dotenv().ok();

    Self {
      database_url: std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set"),

      port: std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a valid number"),

      jwt_secret: std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set"),

      cors_allowed_origins: std::env::var("CORS_ALLOWED_ORIGINS")
        .map(|raw| {
          raw.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
        })
        .unwrap_or_default(),
    }
  }
}