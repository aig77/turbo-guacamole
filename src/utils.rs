use super::config::BASE62;
use axum::http::StatusCode;
use rand::Rng;

pub fn generate_random_base62_code(length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..62);
            BASE62[idx] as char
        })
        .collect()
}

pub fn shortened_url_from_code(code: &str, service_host: &str) -> String {
    format!("{}/{}", service_host, code)
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
