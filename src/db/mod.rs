pub mod queries;

use crate::config::Config;
use sqlx::postgres::{PgPool, PgPoolOptions};

/// PostgreSQL unique constraint violation error code
/// Reference: https://www.postgresql.org/docs/current/errcodes-appendix.html
pub const PG_UNIQUE_VIOLATION: &str = "23505";

pub async fn setup_database(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().connect(&config.database_url).await
}

pub fn is_collision(db_err: &dyn sqlx::error::DatabaseError) -> bool {
    // Unique constraint violation
    db_err.code().is_some_and(|c| c == PG_UNIQUE_VIOLATION)
}

#[macro_export]
macro_rules! sql_query {
    ($module:literal, $file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/sql/",
            $module,
            "/",
            $file,
            ".sql"
        ))
    };
}
