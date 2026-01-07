use crate::{
    config::AppState,
    db::queries::{clicks, urls},
    utils::internal_error,
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
    let result = urls::find_url_by_code(&state.pool, &code).await;

    match result {
        Ok(Some(url)) => {
            info!("Redirect target found");

            if let Err(e) = clicks::insert(&state.pool, &code).await {
                error!("Failed to record click analytics: {}", e);
            }

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
