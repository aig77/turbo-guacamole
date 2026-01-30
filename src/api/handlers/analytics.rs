use crate::{api::internal_error, db::queries::clicks, state::AppState};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;
use std::sync::Arc;
use tracing::instrument;

#[derive(Serialize, Debug)]
pub struct AnalyticsResponse {
    code: String,
    total_clicks: i64,
    daily_clicks: Vec<clicks::DailyClick>,
}

#[instrument(skip(state), fields(code = %code))]
pub async fn analytics(
    Path(code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<AnalyticsResponse>), (StatusCode, String)> {
    let total_clicks = clicks::get_code_total_clicks(&state.pool, &code)
        .await
        .map_err(internal_error)?;

    let daily_clicks = clicks::get_code_daily_clicks(&state.pool, &code)
        .await
        .map_err(internal_error)?;

    let response = AnalyticsResponse {
        code,
        total_clicks,
        daily_clicks,
    };

    Ok((StatusCode::OK, Json(response)))
}
