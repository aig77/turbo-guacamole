use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// PostgreSQL unique constraint violation error code
/// Reference: https://www.postgresql.org/docs/current/errcodes-appendix.html
pub const PG_UNIQUE_VIOLATION: &str = "23505";

pub const CODE_LEN: usize = 6;
pub const BASE62: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub service_host: String,
    pub service_port: String,
    pub admin_username: String,
    pub admin_password: String,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        dotenvy::dotenv().ok();

        Ok(Self {
            service_host: std::env::var("SERVICE_HOST").unwrap_or("127.0.0.1".to_string()),
            service_port: std::env::var("SERVICE_PORT").unwrap_or("3000".to_string()),
            database_url: std::env::var("DATABASE_URL")?,
            admin_username: std::env::var("ADMIN_USERNAME")?,
            admin_password: std::env::var("ADMIN_PASSWORD")?,
        })
    }
}

pub fn setup_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

pub async fn setup_database(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().connect(&config.database_url).await
}

#[warn(dead_code)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}
