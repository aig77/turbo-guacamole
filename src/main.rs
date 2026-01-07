use turbo_guacamole::{config, handlers};

use axum::{
    Router,
    routing::{get, post},
};
use config::{AppState, Config, setup_database, setup_tracing};
use handlers::{admin::admin_routes, redirect::redirect_url, shorten::shorten_url};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    setup_tracing();

    let config = Config::from_env()?;

    info!(
        service_host = %config.service_host,
        service_port = %config.service_port,
        database_url = %config.database_url,
        "Server configuration loaded"
    );

    // set up connection pool
    let pool = setup_database(&config).await?;

    info!("Database connection established");

    let app_state = Arc::new(AppState {
        pool,
        config: config.clone(),
    });

    let app = Router::new()
        .route("/{code}", get(redirect_url))
        .route("/shorten", post(shorten_url))
        .nest("/admin", admin_routes())
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(Arc::clone(&app_state));

    let addr = format!("{}:{}", &config.service_host, &config.service_port);

    info!("Server listening on {}", &addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(listener, app).await?;

    Ok(())
}
