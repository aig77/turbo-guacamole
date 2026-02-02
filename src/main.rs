use turbo_guacamole::{api, cache, config, db, state::AppState};

use std::{net::SocketAddr, sync::Arc};
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    setup_tracing();

    let config = config::Config::from_env();

    info!(
        "Server configuration loaded: host={}, port={}, db={}, cache={}, code_rate_limit={:?}, shorten_rate_limit={:?}",
        config.service_host,
        config.service_port,
        config.database_url,
        config.cache_url,
        config.code_rate_limit_config,
        config.shorten_rate_limit_config,
    );

    // set up postgres connection pool
    let pool = db::setup_database(&config.database_url).await?;
    info!("Postgres connection established");

    // set up redis connection pool
    let redis_pool = cache::setup_cache(&config.cache_url).await?;
    info!("Redis connection established");

    let app_state = Arc::new(AppState {
        pool,
        redis_pool,
        config: config.clone(),
    });

    let app = api::configure(
        &config.code_rate_limit_config,
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
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

fn setup_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
