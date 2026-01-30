use crate::config::RateLimitConfig;
use axum::body::Body;
use governor::middleware::NoOpMiddleware;
use std::{sync::Arc, time::Duration};
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor,
};

pub fn setup_rate_limiter(
    config: &RateLimitConfig,
) -> GovernorLayer<PeerIpKeyExtractor, NoOpMiddleware, Body> {
    tracing::info!(
        "rate limit config -> requests per second: {}, burst size: {}, cleanup interval secs: {}",
        config.requests_per_second,
        config.burst_size,
        config.cleanup_interval_secs
    );

    // Allow bursts with up to five requests per IP address
    // and replenishes one element every two seconds
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(config.requests_per_second)
            .burst_size(config.burst_size)
            .finish()
            .unwrap(),
    );

    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(config.cleanup_interval_secs);
    // a separate background task to clean up
    tokio::spawn(async move {
        let mut interval_timer = tokio::time::interval(interval);
        loop {
            interval_timer.tick().await;
            tracing::info!("rate limiting storage size: {}", governor_limiter.len());
            governor_limiter.retain_recent();
        }
    });

    GovernorLayer::new(governor_conf)
}
