use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use std::sync::Arc;
use tracing::{debug, warn};

pub async fn health(State(state): State<Arc<AppState>>) -> StatusCode {
    let pg_ok = sqlx::query("SELECT 1")
        .execute(&state.pg_pool)
        .await
        .is_ok();

    let redis_ok = match state.redis_pool.get().await {
        Ok(mut conn) => redis::cmd("PING")
            .query_async::<String>(&mut *conn)
            .await
            .is_ok(),
        Err(_) => false,
    };

    if pg_ok && redis_ok {
        debug!("healthy");
        StatusCode::OK
    } else {
        warn!(pg_ok, redis_ok, "unhealthy");
        StatusCode::SERVICE_UNAVAILABLE
    }
}
