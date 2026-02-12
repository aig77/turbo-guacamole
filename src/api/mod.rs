use crate::config::RateLimitConfig;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, services::ServeFile};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod handlers;
mod middleware;

#[derive(OpenApi)]
#[openapi(
      paths(
          handlers::shorten::shorten_url,
          handlers::redirect::redirect_url,
          handlers::analytics::get_stats,
          handlers::analytics::get_code_stats,
          handlers::health::health,
      ),
      components(
          schemas(
              handlers::shorten::ShortenPayload,
              handlers::shorten::ShortenResponse,
              handlers::analytics::StatsResponse,
              handlers::analytics::CodeStatsResponse,
              crate::db::queries::clicks::DailyClick,
          )
      ),
      tags(
          (name = "urls", description = "URL shortening and redirect operations"),
          (name = "analytics", description = "URL shortening and redirect analytics"),
          (name = "health", description = "Health check endpoints")
      ),
      info(
          title = "Turbo Guacamole URL Shortener",
          version = "0.1.0",
          description = "A high-performance URL shortening service"
      )
  )]
pub struct ApiDoc;

pub fn configure(
    redirect_rate_limit_config: &RateLimitConfig,
    shorten_rate_limit_config: &RateLimitConfig,
) -> Router<Arc<AppState>> {
    let redirect_rate_limit =
        middleware::rate_limit::setup_rate_limiter(redirect_rate_limit_config);
    let shorten_rate_limit = middleware::rate_limit::setup_rate_limiter(shorten_rate_limit_config);
    let default_rate_limit =
        middleware::rate_limit::setup_rate_limiter(&RateLimitConfig::default());

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route_service("/", ServeFile::new("static/index.html"))
        // Main routes
        .route(
            "/{code}",
            get(handlers::redirect::redirect_url).layer(redirect_rate_limit),
        )
        .route(
            "/shorten",
            post(handlers::shorten::shorten_url).layer(shorten_rate_limit),
        )
        // Default rate limited routes
        .merge(
            Router::new()
                .route("/health", get(handlers::health::health))
                .route("/stats", get(handlers::analytics::get_stats))
                .route("/{code}/stats", get(handlers::analytics::get_code_stats))
                .layer(default_rate_limit),
        )
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn(
                    middleware::tracing::tracing_middleware,
                ))
                .layer(CorsLayer::permissive())
                .layer(CompressionLayer::new()),
        )
}
