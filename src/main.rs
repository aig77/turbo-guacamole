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
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

const BASE62: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const CODE_LEN: usize = 6;

/// PostgreSQL unique constraint violation error code
/// Reference: https://www.postgresql.org/docs/current/errcodes-appendix.html
const PG_UNIQUE_VIOLATION: &str = "23505";

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let db_connection_str =
        std::env::var("DATABASE_URL").expect("Environment variable DATABASE_URL");

    let base_url = std::env::var("BASE_URL").unwrap_or("127.0.0.1:3000".to_string());

    let admin_username =
        std::env::var("ADMIN_USERNAME").expect("Environment variable ADMIN_USERNAME");

    let admin_password =
        std::env::var("ADMIN_PASSWORD").expect("Environment variable ADMIN_PASSWORD");

    info!(
        base_url = %base_url,
        "Server configuration loaded"
    );

    // set up connection pool
    let pool = PgPoolOptions::new()
        .connect(&db_connection_str)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to connect to database: {}", e);
            panic!("Database connection failed");
        });

    info!("Database connection established");

    let shared_state = Arc::new(AppState {
        pool,
        base_url,
        admin_username,
        admin_password,
    });

    let app = Router::new()
        .route("/{code}", get(redirect))
        .route("/shorten", post(shorten_url))
        .nest("/admin", admin_routes())
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(Arc::clone(&shared_state));

    let listener = tokio::net::TcpListener::bind(&shared_state.base_url)
        .await
        .unwrap();
    info!("Server listening on {}", listener.local_addr().unwrap());
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

#[instrument(skip(state), fields(code = %code))]
async fn redirect(
    Path(code): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Redirect, (StatusCode, String)> {
    let result: Result<Option<String>, _> =
        sqlx::query_scalar("SELECT url FROM urls WHERE code = $1")
            .bind(&code)
            .fetch_optional(&state.pool)
            .await;

    match result {
        Ok(Some(url)) => {
            info!("Redirect target found");

            if let Err(e) = sqlx::query("INSERT INTO clicks (code) VALUES ($1)")
                .bind(&code)
                .execute(&state.pool)
                .await
            {
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

#[instrument(skip(state), fields(url = %payload.url))]
async fn shorten_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ShortenPayload>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    fn validate(url: &str) -> Result<(), (StatusCode, String)> {
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

    validate(&payload.url)?;

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
        let shortened = shortened_url_from_code(&code, &state.base_url);
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
                let shortened = shortened_url_from_code(&code, &state.base_url);
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

fn admin_routes() -> Router<Arc<AppState>> {
    #[instrument(skip(state))]
    async fn list_codes(
        TypedHeader(Authorization(creds)): TypedHeader<Authorization<Basic>>,
        State(state): State<Arc<AppState>>,
    ) -> Result<Json<HashMap<String, String>>, StatusCode> {
        authenticate(
            creds.username(),
            creds.password(),
            &state.admin_username,
            &state.admin_password,
        )?;

        let urls: HashMap<String, String> = sqlx::query_as("SELECT code, url FROM urls")
            .fetch_all(&state.pool)
            .await
            .map_err(|e| {
                error!("Database query failed while listing codes: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .into_iter()
            .collect();

        info!("Retrieved {} URL mappings", urls.len());
        Ok(Json(urls))
    }

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
            .map_err(|e| {
                error!("Database delete failed while removing all codes: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let count = result.rows_affected();
        info!("Deleted {} rows", count);
        Ok(format!("Deleted {} rows", count))
    }

    #[instrument(skip(creds, state), fields(code = %code))]
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
            Ok(Some(url)) => {
                info!("Code successfully deleted");
                Ok(url)
            }
            Ok(None) => {
                warn!("Code not found for deletion");
                Err(StatusCode::NOT_FOUND)
            }
            Err(e) => {
                error!("Database delete failed while removing code: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    fn authenticate(
        input_username: &str,
        input_password: &str,
        admin_username: &str,
        admin_password: &str,
    ) -> Result<(), StatusCode> {
        if input_username == admin_username && input_password == admin_password {
            debug!("Authentication successful");
            Ok(())
        } else {
            warn!("Authentication failed");
            Err(StatusCode::UNAUTHORIZED)
        }
    }

    Router::new()
        .route("/codes", get(list_codes))
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
    format!("{}/{}", base_url, code)
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
