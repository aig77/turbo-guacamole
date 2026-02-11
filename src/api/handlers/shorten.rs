use crate::{
    cache::add_to_cache,
    db::{is_collision, queries::urls},
    error::{ApiError, ApiResult},
    state::AppState,
};
use axum::{Json, extract::State, http::StatusCode};
use rand::Rng;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};
use url::Url;

const URL_LENGTH_LIMIT: usize = 2048;
const BASE62: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const CODE_LEN: usize = 6;
const MAX_COLLISION_RETRIES: usize = 5;

#[derive(Debug, serde::Deserialize)]
pub struct ShortenPayload {
    pub url: String,
}

#[instrument(skip(state), fields(url = %payload.url))]
pub async fn shorten_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ShortenPayload>,
) -> ApiResult<(StatusCode, String)> {
    if payload.url.len() > URL_LENGTH_LIMIT {
        warn!("URL exceeds limit of {} characters", URL_LENGTH_LIMIT);
        return Err(ApiError::UrlTooLong {
            max: URL_LENGTH_LIMIT,
        });
    }

    validate_url_format(&payload.url)?;

    let addr = format!(
        "{}:{}",
        &state.config.service_host, &state.config.service_port
    );

    // Check if this URL has already been shortened (duplicate detection)
    let existing = urls::find_code_by_url(&state.pg_pool, &payload.url).await?;

    if let Some(code) = existing {
        info!(
            "URL already exists, returning from existing code: {}",
            &code
        );
        add_to_cache(&state.redis_pool, &code, &payload.url).await;
        let shortened = shortened_url_from_code(&code, &addr);
        return Ok((StatusCode::OK, shortened));
    }

    for _ in 0..MAX_COLLISION_RETRIES {
        let code = generate_random_base62_code(CODE_LEN);
        debug!("Code generated: {}", &code);

        match urls::insert(&state.pg_pool, &code, &payload.url).await {
            Ok(_) => {
                info!("Short URL created with code: {}", &code);
                add_to_cache(&state.redis_pool, &code, &payload.url).await;
                let shortened = shortened_url_from_code(&code, &addr);
                return Ok((StatusCode::CREATED, shortened));
            }
            Err(sqlx::Error::Database(db_err)) if is_collision(db_err.as_ref()) => {
                warn!("Collision - retrying with new code");
                continue;
            }
            Err(e) => {
                error!("Database insert failed while creating short URL: {}", e);
                return Err(ApiError::Database(e));
            }
        }
    }

    Err(ApiError::TooManyCollisions)
}

fn validate_url_format(url: &str) -> ApiResult<()> {
    let parsed = Url::parse(url).map_err(|e| {
        warn!("Invalid URL format: {}", e);
        ApiError::InvalidUrl(e)
    })?;

    if parsed.scheme() == "http" || parsed.scheme() == "https" {
        Ok(())
    } else {
        warn!(
            scheme = %parsed.scheme(),
            "Rejected URL with unsupported scheme (only http/https allowed)"
        );
        Err(ApiError::UnsupportedScheme {
            scheme: parsed.scheme().to_string(),
        })
    }
}

fn generate_random_base62_code(length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..62);
            BASE62[idx] as char
        })
        .collect()
}

fn shortened_url_from_code(code: &str, service_host: &str) -> String {
    format!("{}/{}", service_host, code)
}
