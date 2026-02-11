use crate::{
    cache::add_to_cache,
    db::queries::{clicks, urls},
    error::{ApiError, ApiResult},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    response::Redirect,
};
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

#[utoipa::path(
    get,
    path = "/v1/{code}",
    params(
        ("code" = String, Path, description = "Short URL code to redirect")
    ),
    responses(
        (status = 200, description = "Redirect successful"),
        (status = 404, description = "URL not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "urls"
)]
#[instrument(skip(state), fields(code = %code))]
pub async fn redirect_url(
    Path(code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> ApiResult<Redirect> {
    // Try to retrieve from cache
    if let Ok(mut conn) = state.redis_pool.get().await
        && let Ok(Some(url)) = redis::cmd("GET")
            .arg(format!("short:{code}"))
            .query_async::<Option<String>>(&mut *conn)
            .await
    {
        info!("Cache hit");

        if let Err(e) = clicks::insert(&state.pg_pool, &code).await {
            error!("Failed to record click analytics: {}", e);
        }

        return Ok(Redirect::temporary(&url));
    }

    // Cache miss, hit postgres
    match urls::find_url_by_code(&state.pg_pool, &code).await {
        Ok(Some(url)) => {
            info!("Cache miss, fetched from db");
            add_to_cache(&state.redis_pool, &code, &url).await;
            if let Err(e) = clicks::insert(&state.pg_pool, &code).await {
                error!("Failed to record click analytics: {}", e);
            }
            info!("Redirecting");
            Ok(Redirect::temporary(&url))
        }
        Ok(None) => {
            warn!("URL not found for code");
            Err(ApiError::NotFound)
        }
        Err(e) => {
            error!(
                "Database query failed while looking up redirect code: {}",
                e
            );
            Err(ApiError::Database(e))
        }
    }
}
