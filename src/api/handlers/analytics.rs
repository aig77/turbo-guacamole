use crate::{db::queries::clicks, error::ApiResult, state::AppState};
use axum::{
    Json,
    extract::{Path, State},
};
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
) -> ApiResult<Json<AnalyticsResponse>> {
    let total_clicks = clicks::get_code_total_clicks(&state.pg_pool, &code).await?;

    let daily_clicks = clicks::get_code_daily_clicks(&state.pg_pool, &code).await?;

    let response = AnalyticsResponse {
        code,
        total_clicks,
        daily_clicks,
    };

    Ok(Json(response))
}
