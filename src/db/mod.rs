pub mod queries;

use crate::sql_query;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::{error, info};

/// PostgreSQL unique constraint violation error code
/// Reference: https://www.postgresql.org/docs/current/errcodes-appendix.html
pub const PG_UNIQUE_VIOLATION: &str = "23505";

pub async fn setup_database(url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().connect(url).await
}

pub fn is_collision(db_err: &dyn sqlx::error::DatabaseError) -> bool {
    // Unique constraint violation
    db_err.code().is_some_and(|c| c == PG_UNIQUE_VIOLATION)
}

pub async fn cleanup_stale_urls(pool: &PgPool, days: i32) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(sql_query!("", "cleanup_stale_urls"))
        .bind(days)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

pub fn start_cleanup_task(pool: PgPool, stale_urls_days: i32) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(86400)).await; // Daily

            match cleanup_stale_urls(&pool, stale_urls_days).await {
                Ok(rows) => info!("Cleaned up {} stale URLs", rows),
                Err(e) => error!("Error cleaning stale URLs: {}", e),
            }
        }
    });
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
