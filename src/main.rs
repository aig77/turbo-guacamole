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
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::{collections::HashMap, sync::Arc};
use url::Url;

const BASE_URL: &str = "tg.com";

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let db_connection_str =
        std::env::var("DATABASE_URL").expect("Environment variable DATABASE_URL");

    // set up connection pool
    let pool = PgPoolOptions::new()
        .connect(&db_connection_str)
        .await
        .expect("can't connect to database");

    let admin_username =
        std::env::var("ADMIN_USERNAME").expect("Environment variable ADMIN_USERNAME");
    let admin_password =
        std::env::var("ADMIN_PASSWORD").expect("Environment variable ADMIN_PASSWORD");

    let shared_state = Arc::new(AppState {
        pool,
        admin_username,
        admin_password,
    });

    let app = Router::new()
        .route("/{code}", get(redirect))
        .route("/shorten", post(shorten_url))
        .route("/keys", get(list_keys))
        .nest("/admin", admin_routes())
        .with_state(Arc::clone(&shared_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    dbg!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

struct AppState {
    // TODO: convert database to Postgres
    pool: PgPool,
    admin_username: String,
    admin_password: String,
}

#[derive(Deserialize)]
struct ShortenPayload {
    url: String,
}

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
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// TODO: collision strategy
async fn shorten_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ShortenPayload>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let normalized_url = normalize_url(&payload.url);
    validate_url(&normalized_url)?;
    let key = encode(&normalized_url);
    let shortened = shortened_url_from_key(&key);
    if !state.read().unwrap().db.contains_key(&key) {
        state.write().unwrap().db.insert(key, normalized_url);
        Ok((StatusCode::CREATED, shortened))
    } else {
        Ok((StatusCode::OK, shortened))
    }
}

async fn list_keys(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    let db = &state.read().unwrap().db;
    // TODO: without cloning
    Ok(Json(db.clone()))
}

fn admin_routes() -> Router<Arc<AppState>> {
    async fn delete_all_keys(
        TypedHeader(Authorization(creds)): TypedHeader<Authorization<Basic>>,
        State(state): State<Arc<AppState>>,
    ) -> Result<String, StatusCode> {
        auth_request(
            creds.username(),
            creds.password(),
            &state.read().unwrap().admin_username,
            &state.read().unwrap().admin_password,
        )?;
        let count = state.write().unwrap().db.drain().count();
        Ok(count.to_string())
    }

    async fn remove_key(
        TypedHeader(Authorization(creds)): TypedHeader<Authorization<Basic>>,
        Path(key): Path<String>,
        State(state): State<Arc<AppState>>,
    ) -> Result<String, StatusCode> {
        auth_request(
            creds.username(),
            creds.password(),
            &state.read().unwrap().admin_username,
            &state.read().unwrap().admin_password,
        )?;
        if let Some(value) = state.write().unwrap().db.remove(&key) {
            Ok(value)
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }

    fn auth_request(
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
        .route("/keys", delete(delete_all_keys))
        .route("/keys/{key}", delete(remove_key))
}

fn encode(s: &str) -> String {
    let hash = Sha256::digest(s);
    let bytes = hash.as_slice();
    let eight_bytes: [u8; 8] = bytes[..8].try_into().unwrap();
    let number = u64::from_be_bytes(eight_bytes);
    base62::encode(number)
}

fn shortened_url_from_key(key: &str) -> String {
    let mut shortened = String::from("https://");
    shortened.push_str(BASE_URL);
    shortened.push('/');
    shortened.push_str(key);
    shortened
}

fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

fn validate_url(url: &str) -> Result<(), (StatusCode, String)> {
    let parsed =
        Url::parse(url).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", e)))?;
    if parsed.scheme() == "http" || parsed.scheme() == "https" {
        Ok(())
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            "Only http and https schemes are accepted".to_string(),
        ))
    }
}
