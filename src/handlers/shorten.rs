use crate::{
    config::{AppState, CODE_LEN, PG_UNIQUE_VIOLATION},
    models::ShortenPayload,
    utils::{generate_random_base62_code, internal_error, shortened_url_from_code},
};
use axum::{Json, extract::State, http::StatusCode};
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};
use url::Url;

#[instrument(skip(state), fields(url = %payload.url))]
pub async fn shorten_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ShortenPayload>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    validate_url_format(&payload.url)?;

    // grab code and url for collision handling later
    let existing: Option<String> = sqlx::query_scalar("SELECT code FROM urls WHERE url = $1")
        .bind(&payload.url)
        .fetch_optional(&state.pool)
        .await
        .map_err(internal_error)?;

    if let Some(code) = existing {
        info!(
            "URL already exists, returning from existing code: {}",
            &code
        );
        let shortened = shortened_url_from_code(&code, &state.config.service_host);
        return Ok((StatusCode::OK, shortened));
    }

    loop {
        let code = generate_random_base62_code(CODE_LEN);
        debug!("Code generated: {}", &code);

        let result = sqlx::query("INSERT INTO urls (code, url) VALUES ($1, $2)")
            .bind(&code)
            .bind(&payload.url)
            .execute(&state.pool)
            .await;

        match result {
            Ok(_) => {
                info!("Short URL created with code: {}", &code);
                let shortened = shortened_url_from_code(&code, &state.config.service_host);
                return Ok((StatusCode::CREATED, shortened));
            }
            Err(sqlx::Error::Database(db_err)) if is_collision(db_err.as_ref()) => {
                warn!("Collision - retrying with new code");
                continue;
            }
            Err(e) => {
                error!("Database insert failed while creating short URL: {}", e);
                return Err(internal_error(e));
            }
        }
    }
}

fn validate_url_format(url: &str) -> Result<(), (StatusCode, String)> {
    let parsed = Url::parse(url).map_err(|e| {
        warn!("Invalid URL format: {}", e);
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid URL format: {}", e),
        )
    })?;

    if parsed.scheme() == "http" || parsed.scheme() == "https" {
        Ok(())
    } else {
        warn!(
            scheme = %parsed.scheme(),
            "Rejected URL with unsupported scheme (only http/https allowed)"
        );
        Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Rejected URL with unsupported scheme (only http/https allowed): {}",
                parsed.scheme()
            ),
        ))
    }
}

fn is_collision(db_err: &dyn sqlx::error::DatabaseError) -> bool {
    // Unique constraint violation
    db_err.code().is_some_and(|c| c == PG_UNIQUE_VIOLATION)
}
