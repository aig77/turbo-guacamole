use crate::config::RateLimitConfig;
use crate::state::AppState;
use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

mod handlers;
mod middleware;
mod v1;

pub fn configure(
    code_rate_limit: &RateLimitConfig,
    shorten_rate_limit: &RateLimitConfig,
) -> Router<Arc<AppState>> {
    Router::new()
        .nest("/v1", v1::configure(code_rate_limit, shorten_rate_limit))
        .route("/health", get(handlers::health::health))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
