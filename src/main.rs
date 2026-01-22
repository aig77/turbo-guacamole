use turbo_guacamole::{config, db, handlers, middleware};

use axum::{
    Router,
    routing::{get, post},
};
use config::{AppState, Config, setup_tracing};
use db::setup_database;
use handlers::{admin::admin_routes, redirect::redirect_url, shorten::shorten_url};
use middleware::rate_limit::setup_rate_limiter;
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    setup_tracing();

    let config = Config::from_env();

    info!(
        "Server configuration loaded: host={}, port={}, db={}, code_rate_limit={:?}, shorten_rate_limit={:?}",
        config.service_host,
        config.service_port,
        config.database_url,
        config.code_rate_limit_config,
        config.shorten_rate_limit_config,
    );

    // set up connection pool
    let pool = setup_database(&config).await?;

    info!("Database connection established");

    let app_state = Arc::new(AppState {
        pool,
        config: config.clone(),
    });

    let app = Router::new()
        .route(
            "/{code}",
            get(redirect_url).layer(setup_rate_limiter(config.code_rate_limit_config)),
        )
        .route(
            "/shorten",
            post(shorten_url).layer(setup_rate_limiter(config.shorten_rate_limit_config)),
        )
        .nest("/admin", admin_routes())
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
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
