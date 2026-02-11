use crate::config::RateLimitConfig;
use crate::state::AppState;
use axum::Router;
use axum::routing::get;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod handlers;
mod middleware;
mod v1;

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
    code_rate_limit: &RateLimitConfig,
    shorten_rate_limit: &RateLimitConfig,
) -> Router<Arc<AppState>> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/v1", v1::configure(code_rate_limit, shorten_rate_limit))
        .route("/health", get(handlers::health::health))
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn(
                    middleware::tracing::tracing_middleware,
                ))
                .layer(CorsLayer::permissive()),
        )
}
