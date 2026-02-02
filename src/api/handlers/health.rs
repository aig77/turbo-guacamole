use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use std::sync::Arc;
use tracing::debug;

pub async fn health(State(state): State<Arc<AppState>>) -> (StatusCode, String) {
    let pg_ok = sqlx::query("SELECT 1").execute(&state.pool).await.is_ok();

    debug!("Postgress ok? {}", pg_ok);

    let redis_ok = state
        .redis_pool
        .get()
        .await
        .map(|mut conn| async move {
            let _ = redis::cmd("PING")
                .query_async::<String>(&mut *conn)
                .await
                .is_ok();
        })
        .is_ok();

    debug!("Redis ok? {}", redis_ok);

    let check_str = format!("Postgres ok? {}\nRedis ok? {}", pg_ok, redis_ok);

    if pg_ok && redis_ok {
        (StatusCode::OK, check_str)
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, check_str)
    }
}
