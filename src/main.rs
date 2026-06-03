use axum::http::{HeaderValue, Method};
use tower_http::{compression::CompressionLayer, cors::{Any, CorsLayer}, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use web_api::{infraestructure::{config::AppConfig, database::database_pool::create_database_pool}, presentation::routes::create_router};

// Restrict CORS to the methods this API actually serves. Origins are restricted
// when CORS_ALLOWED_ORIGINS is set; otherwise the layer is permissive (dev only).
fn build_cors(allowed_origins: &[String]) -> CorsLayer {
  let cors = CorsLayer::new()
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers(Any);

  if allowed_origins.is_empty() {
    cors.allow_origin(Any)
  } else {
    let origins: Vec<HeaderValue> = allowed_origins
      .iter()
      .filter_map(|o| o.parse().ok())
      .collect();
    cors.allow_origin(origins)
  }
}

#[tokio::main]
async fn main() {
  tracing_subscriber::registry()
    .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
    .with(tracing_subscriber::fmt::layer())
    .init();

  let config = AppConfig::from_env();
  let db_pool = create_database_pool(&config.database_url).await;

  sqlx::migrate!("./migrations")
    .run(&db_pool)
    .await
    .expect("Failed to run database migrations");

  let app = create_router(db_pool)
    .layer(
      // Tower layers compose from outermost to innermost. The request
      // passes through each layer before reaching the handler.
      tower::ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())   // logs each request
        .layer(CompressionLayer::new())       // gzip responses
        .layer(build_cors(&config.cors_allowed_origins)),
    );

  let address = format!("0.0.0.0:{}", config.port);
  let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
  
  let result = axum::serve(listener, app).await;

  if let Err(err) = result {
    eprintln!("Server error: {}", err);
  }
}
