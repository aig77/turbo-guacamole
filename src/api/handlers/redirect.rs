use crate::{
    api::internal_error,
    db::queries::{clicks, urls},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
};
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

#[instrument(skip(state), fields(code = %code))]
pub async fn redirect_url(
    Path(code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Redirect, (StatusCode, String)> {
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

            if let Ok(mut conn) = state.redis_pool.get().await {
                let _ = redis::cmd("SET")
                    .arg(format!("short:{code}"))
                    .arg(&url)
                    .arg("EX")
                    .arg(3600)
                    .query_async::<()>(&mut *conn)
                    .await;

                info!("Inserted into cache");
            }

            if let Err(e) = clicks::insert(&state.pg_pool, &code).await {
                error!("Failed to record click analytics: {}", e);
            }

            info!("Redirecting");

            Ok(Redirect::temporary(&url))
        }
        Ok(None) => {
            warn!("URL not found for code");
            Err((StatusCode::NOT_FOUND, "URL not found".to_string()))
        }
        Err(e) => {
            error!(
                "Database query failed while looking up redirect code: {}",
                e
            );
            Err(internal_error(e))
        }
    }
}
