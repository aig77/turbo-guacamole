use crate::{
    cache,
    db::queries::{clicks, stats},
    error::{ApiError, ApiResult},
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use std::sync::Arc;
use tracing::instrument;
use utoipa::ToSchema;

#[derive(Serialize, Debug, ToSchema)]
pub struct StatsResponse {
    total_urls: i64,
    total_clicks: i64,
}

#[utoipa::path(
    get,
    path = "/stats",
    responses(
        (status = 200, description = "Stats retrieved successfully", body = StatsResponse),
        (status = 500, description = "Internal server error"),
    ),
    tag = "analytics"
)]
#[instrument(skip(state))]
pub async fn get_stats(State(state): State<Arc<AppState>>) -> ApiResult<Json<StatsResponse>> {
    // Try cache first
    if let Some((total_urls, total_clicks)) = cache::get_stats(&state.redis_pool).await {
        let response = StatsResponse {
            total_urls,
            total_clicks,
        };
        return Ok(Json(response));
    }

    // Cache miss - DB
    match stats::get_total_counts(&state.pg_pool).await {
        Ok((total_urls, total_clicks)) => {
            // Cache for 5 minutes
            cache::set_stats(&state.redis_pool, total_urls, total_clicks, 300).await;
            let response = StatsResponse {
                total_urls,
                total_clicks,
            };
            Ok(Json(response))
        }
        Err(e) => Err(ApiError::Database(e)),
    }
}

#[derive(Serialize, Debug, ToSchema)]
pub struct CodeStatsResponse {
    code: String,
    total_clicks: i64,
    daily_clicks: Vec<clicks::DailyClick>,
}

#[utoipa::path(
    get,
    path = "/{code}/stats",
    params(
        ("code" = String, Path, description = "Short URL code")
    ),
    responses(
        (status = 200, description = "Analytics retrieved successfully", body = CodeStatsResponse),
        (status = 404, description = "URL code not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "analytics"
)]
#[instrument(skip(state), fields(code = %code))]
pub async fn get_code_stats(
    Path(code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<CodeStatsResponse>> {
    let total_clicks = clicks::get_code_total_clicks(&state.pg_pool, &code).await?;

    let daily_clicks = clicks::get_code_daily_clicks(&state.pg_pool, &code).await?;

    let response = CodeStatsResponse {
        code,
        total_clicks,
        daily_clicks,
    };

    Ok(Json(response))
}
