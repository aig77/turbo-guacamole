use turbo_guacamole::{api, cache, config, db, state::AppState, utils};

use std::{net::SocketAddr, sync::Arc};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    utils::setup_tracing();

    let config = config::Config::from_env();

    info!(
        service_host = %config.service_host,
        service_port = %config.service_port,
        database_url = %utils::truncate(&config.database_url, 15),
        cache_url = %utils::truncate(&config.cache_url, 15),
        stale_url_days = %config.stale_urls_days,
    );

    // set up postgres connection pool
    let pg_pool = db::setup_database(&config.database_url).await?;
    info!("Postgres connection established");

    // start stale URL cleanup task
    db::start_cleanup_task(pg_pool.clone(), config.stale_urls_days);

    // set up redis connection pool
    let redis_pool = cache::setup_cache(&config.cache_url).await?;
    info!("Redis connection established");

    let app_state = Arc::new(AppState {
        pg_pool,
        redis_pool,
        config: config.clone(),
    });

    let app = api::configure(
        &config.redirect_rate_limit_config,
        &config.shorten_rate_limit_config,
    )
    .with_state(Arc::clone(&app_state));

    let addr = format!("{}:{}", &config.service_host, &config.service_port);

    info!("Server listening on {}", &addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(utils::shutdown_signal())
    .await?;

    Ok(())
}
