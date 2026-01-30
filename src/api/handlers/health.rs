use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use std::sync::Arc;

pub async fn health(State(state): State<Arc<AppState>>) -> StatusCode {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}
