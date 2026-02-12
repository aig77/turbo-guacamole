use crate::config::RateLimitConfig;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod handlers;
mod middleware;

#[derive(OpenApi)]
#[openapi(
      paths(
          handlers::shorten::shorten_url,
          handlers::redirect::redirect_url,
          handlers::analytics::analytics,
          handlers::health::health,
      ),
      components(
          schemas(
              handlers::shorten::ShortenPayload,
              handlers::analytics::AnalyticsResponse,
              crate::db::queries::clicks::DailyClick
          )
      ),
      tags(
          (name = "urls", description = "URL shortening and redirect operations"),
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
    let analytics_rate_limit =
        middleware::rate_limit::setup_rate_limiter(&RateLimitConfig::default());
    let shorten_rate_limit = middleware::rate_limit::setup_rate_limiter(shorten_rate_limit_config);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route(
            "/{code}",
            get(handlers::redirect::redirect_url).layer(redirect_rate_limit),
        )
        .route(
            "/shorten",
            post(handlers::shorten::shorten_url).layer(shorten_rate_limit),
        )
        .route(
            "/{code}/stats",
            get(handlers::analytics::analytics).layer(analytics_rate_limit),
        )
        .route("/health", get(handlers::health::health))
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn(
                    middleware::tracing::tracing_middleware,
                ))
                .layer(CorsLayer::permissive()),
        )
}
