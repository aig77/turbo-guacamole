use super::handlers;
use crate::api::middleware::rate_limit::setup_rate_limiter;
use crate::config::RateLimitConfig;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

pub fn configure(
    code_rate_limit: &RateLimitConfig,
    shorten_rate_limit: &RateLimitConfig,
) -> Router<Arc<AppState>> {
    let code_rate_limit = setup_rate_limiter(code_rate_limit);
    let shorten_rate_limit = setup_rate_limiter(shorten_rate_limit);

    Router::new()
        .route(
            "/{code}",
            get(handlers::redirect::redirect_url).layer(code_rate_limit.clone()),
        )
        .route(
            "/{code}/stats",
            get(handlers::analytics::analytics).layer(code_rate_limit),
        )
        .route(
            "/shorten",
            post(handlers::shorten::shorten_url).layer(shorten_rate_limit),
        )
}
