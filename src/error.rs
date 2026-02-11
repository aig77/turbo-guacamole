use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("URL exceeds maximum length of {max} characters")]
    UrlTooLong { max: usize },

    #[error("Invalid URL format: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Unsupported URL scheme: {scheme}. Only http/https allowed")]
    UnsupportedScheme { scheme: String },

    #[error("URL not found")]
    NotFound,

    #[error("Maximum collision retries exceeded")]
    TooManyCollisions,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Failed to record click analytics: {0}")]
    ClickTrackingFailed(#[source] sqlx::Error),

    #[error("Cache error: {0}")]
    Cache(#[from] redis::RedisError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::UrlTooLong { max } => (
                StatusCode::BAD_REQUEST,
                format!("URL exceeds maximum length of {max} characters"),
            ),
            ApiError::InvalidUrl(e) => {
                (StatusCode::BAD_REQUEST, format!("Invalid URL format: {e}"))
            }
            ApiError::UnsupportedScheme { scheme } => (
                StatusCode::BAD_REQUEST,
                format!("Unsupported URL scheme: {scheme}"),
            ),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "URL not found".to_string()),
            ApiError::TooManyCollisions => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Service temporarily unavailable".to_string(),
            ),
            ApiError::Database(e) => {
                tracing::error!("Database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            ApiError::ClickTrackingFailed(e) => {
                tracing::error!("Database error while trying to insert click: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            ApiError::Cache(e) => {
                tracing::error!("Cache error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
