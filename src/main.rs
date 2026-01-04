use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
    routing::{delete, get, post},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Basic},
};
use rand::Rng;
use serde::Deserialize;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, error, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

const BASE62: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const CODE_LEN: usize = 6;

/// PostgreSQL unique constraint violation error code
/// Reference: https://www.postgresql.org/docs/current/errcodes-appendix.html
const PG_UNIQUE_VIOLATION: &str = "23505";

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let db_connection_str =
        std::env::var("DATABASE_URL").expect("Environment variable DATABASE_URL");

    let base_url = std::env::var("BASE_URL").unwrap_or("127.0.0.1:3000".to_string());

    let admin_username =
        std::env::var("ADMIN_USERNAME").expect("Environment variable ADMIN_USERNAME");

    let admin_password =
        std::env::var("ADMIN_PASSWORD").expect("Environment variable ADMIN_PASSWORD");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // set up connection pool
    let pool = PgPoolOptions::new()
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");

    let shared_state = Arc::new(AppState {
        pool,
        base_url,
        admin_username,
        admin_password,
    });

    let app = Router::new()
        .route("/{code}", get(redirect))
        .route("/shorten", post(shorten_url))
        .route("/codes", get(list_codes))
        .nest("/admin", admin_routes())
        .with_state(Arc::clone(&shared_state));

    let listener = tokio::net::TcpListener::bind(&shared_state.base_url)
        .await
        .unwrap();
    debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

struct AppState {
    pool: PgPool,
    base_url: String,
    admin_username: String,
    admin_password: String,
}

#[derive(Debug, Deserialize)]
struct ShortenPayload {
    url: String,
}

#[instrument(skip(state))]
async fn redirect(
    Path(code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Redirect, (StatusCode, String)> {
    let result: Result<Option<String>, _> =
        sqlx::query_scalar("SELECT url FROM urls WHERE code = $1")
            .bind(code)
            .fetch_optional(&state.pool)
            .await;

    match result {
        Ok(Some(url)) => Ok(Redirect::temporary(&url)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "no code found".to_string())),
        Err(e) => Err(internal_error(e)),
    }
}

#[instrument(skip(state))]
async fn shorten_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ShortenPayload>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    fn validate(url: &str) -> Result<(), (StatusCode, String)> {
        let parsed = Url::parse(url)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", e)))?;

        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err((
                StatusCode::BAD_REQUEST,
                "Only http and https schemes are accepted".to_string(),
            ));
        }

        Ok(())
    }

    fn is_collision(db_err: &dyn sqlx::error::DatabaseError) -> bool {
        // Unique constraint violation
        db_err.code().is_some_and(|c| c == PG_UNIQUE_VIOLATION)
    }

    validate(&payload.url)?;

    // grab code and url for collision handling later
    let existing: Option<String> = sqlx::query_scalar("SELECT code FROM urls WHERE url = $1")
        .bind(&payload.url)
        .fetch_optional(&state.pool)
        .await
        .map_err(internal_error)?;

    if let Some(code) = existing {
        let shortened = shortened_url_from_code(&code, &state.base_url);
        return Ok((StatusCode::OK, shortened));
    }

    loop {
        let code = generate_random_base62_code(CODE_LEN);

        let result = sqlx::query("INSERT INTO urls (code, url) VALUES ($1, $2)")
            .bind(&code)
            .bind(&payload.url)
            .execute(&state.pool)
            .await;

        match result {
            Ok(_) => {
                let shortened = shortened_url_from_code(&code, &state.base_url);
                return Ok((StatusCode::CREATED, shortened));
            }
            Err(sqlx::Error::Database(db_err)) if is_collision(db_err.as_ref()) => {
                tracing::debug!("Collision - retrying with new code");
                continue;
            }
            Err(e) => return Err(internal_error(e)),
        }
    }
}

#[instrument(skip(state))]
async fn list_codes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    let urls = sqlx::query_as("SELECT code, url FROM urls")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .collect();

    Ok(Json(urls))
}

fn admin_routes() -> Router<Arc<AppState>> {
    #[instrument(skip(creds, state))]
    async fn delete_all_codes(
        TypedHeader(Authorization(creds)): TypedHeader<Authorization<Basic>>,
        State(state): State<Arc<AppState>>,
    ) -> Result<String, StatusCode> {
        authenticate(
            creds.username(),
            creds.password(),
            &state.admin_username,
            &state.admin_password,
        )?;

        let result = sqlx::query("DELETE FROM urls")
            .execute(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let count = result.rows_affected();
        Ok(format!("Deleted {} rows", count))
    }

    #[instrument(skip(creds, state))]
    async fn remove_codes(
        TypedHeader(Authorization(creds)): TypedHeader<Authorization<Basic>>,
        Path(code): Path<String>,
        State(state): State<Arc<AppState>>,
    ) -> Result<String, StatusCode> {
        authenticate(
            creds.username(),
            creds.password(),
            &state.admin_username,
            &state.admin_password,
        )?;

        let result = sqlx::query_scalar("DELETE FROM urls where code = $1 RETURNING url")
            .bind(&code)
            .fetch_optional(&state.pool)
            .await;

        match result {
            Ok(Some(url)) => Ok(url),
            Ok(None) => Err(StatusCode::NOT_FOUND),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    fn authenticate(
        input_username: &str,
        input_password: &str,
        admin_username: &str,
        admin_password: &str,
    ) -> Result<(), StatusCode> {
        if input_username == admin_username && input_password == admin_password {
            Ok(())
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }

    Router::new()
        .route("/codes", delete(delete_all_codes))
        .route("/codes/{code}", delete(remove_codes))
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

fn shortened_url_from_code(code: &str, base_url: &str) -> String {
    let mut shortened = String::from(base_url);
    shortened.push('/');
    shortened.push_str(code);
    shortened
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
