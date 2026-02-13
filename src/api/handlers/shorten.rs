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
use utoipa::ToSchema;

const URL_LENGTH_LIMIT: usize = 2048;
const BASE62: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const CODE_LEN: usize = 6;
const MAX_COLLISION_RETRIES: usize = 5;

#[derive(Debug, serde::Deserialize, ToSchema)]
pub struct ShortenPayload {
    /// The URL to shorten
    #[schema(
        example = "https://example.com",
        max_length = 2048,
        pattern = "^https?://.*",
        format = "uri"
    )]
    pub url: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct ShortenResponse {
    /// The generated short code
    pub code: String,
}

#[utoipa::path(
    post,
    path = "/shorten",
    request_body = ShortenPayload,
    responses(
        (status = 200, description = "URL already exists", body = ShortenResponse),
        (status = 201, description = "URL shortened successfully", body = ShortenResponse),
        (status = 400, description = "Invalid URL or URL too long"),
        (status = 500, description = "Internal server error")
    ),
    tag = "urls"
)]
#[instrument(skip(state))]
pub async fn shorten_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ShortenPayload>,
) -> ApiResult<(StatusCode, Json<ShortenResponse>)> {
    if payload.url.len() > URL_LENGTH_LIMIT {
        warn!("URL exceeds limit of {} characters", URL_LENGTH_LIMIT);
        return Err(ApiError::UrlTooLong {
            max: URL_LENGTH_LIMIT,
        });
    }

    validate_url_format(&payload.url)?;

    // Check if this URL has already been shortened (duplicate detection)
    let existing = urls::find_code_by_url(&state.pg_pool, &payload.url).await?;

    if let Some(code) = existing {
        info!(
            "URL already exists, returning from existing code: {}",
            &code
        );
        add_to_cache(&state.redis_pool, &code, &payload.url).await;
        return Ok((StatusCode::OK, Json(ShortenResponse { code })));
    }

    for _ in 0..MAX_COLLISION_RETRIES {
        let code = generate_random_base62_code(CODE_LEN);
        debug!("Code generated: {}", &code);

        match urls::insert(&state.pg_pool, &code, &payload.url).await {
            Ok(_) => {
                info!("Short URL created with code: {}", &code);
                add_to_cache(&state.redis_pool, &code, &payload.url).await;
                return Ok((StatusCode::CREATED, Json(ShortenResponse { code })));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_format_valid_http() {
        let url = "http://doc.rust-lang.org/book/";
        assert!(validate_url_format(url).is_ok());
    }

    #[test]
    fn test_validate_url_format_valid_https() {
        let url = "https://doc.rust-lang.org/book/";
        assert!(validate_url_format(url).is_ok());
    }

    #[test]
    fn test_validate_url_format_empty() {
        let url = "";
        let result = validate_url_format(url);
        assert!(matches!(result, Err(ApiError::InvalidUrl(_))));
    }

    #[test]
    fn test_validate_url_format_invalid_url() {
        let url = "not a url at all";
        let result = validate_url_format(url);
        assert!(matches!(result, Err(ApiError::InvalidUrl(_))));
    }

    #[test]
    fn test_validate_url_format_file_scheme() {
        let url = "file://example";
        let result = validate_url_format(url);
        match result {
            Err(ApiError::UnsupportedScheme { scheme }) => assert_eq!(scheme, "file"),
            _ => panic!("Expected UnsupportedScheme error"),
        }
    }

    #[test]
    fn test_generate_random_base62_code_length() {
        let code = generate_random_base62_code(10);
        assert_eq!(code.len(), 10);
    }

    #[test]
    fn test_generate_random_base62_code_is_alphanumeric() {
        let code = generate_random_base62_code(10);
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
