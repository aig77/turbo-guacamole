use turbo_guacamole::{api, config, db, state::AppState};

use std::{net::SocketAddr, sync::Arc};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    setup_tracing();

    let config = config::Config::from_env();

    info!(
        "Server configuration loaded: host={}, port={}, db={}, code_rate_limit={:?}, shorten_rate_limit={:?}",
        config.service_host,
        config.service_port,
        config.database_url,
        config.code_rate_limit_config,
        config.shorten_rate_limit_config,
    );

    // set up connection pool
    let pool = db::setup_database(&config).await?;

    info!("Database connection established");

    let app_state = Arc::new(AppState {
        pool,
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
