use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
    routing::{delete, get, post},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use url::Url;

const BASE_URL: &str = "tg.com";

#[tokio::main]
async fn main() {
    let shared_state = SharedState::default();

    let app = Router::new()
        .route("/{key}", get(redirect))
        .route("/shorten", post(shorten_url))
        .route("/keys", get(list_keys))
        // TODO: admin keys
        .nest("/admin", admin_routes())
        .with_state(Arc::clone(&shared_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    dbg!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

// TODO: handle unwraps for app state
type SharedState = Arc<RwLock<AppState>>;

#[derive(Default)]
struct AppState {
    // TODO: convert database to Postgres
    db: HashMap<String, String>,
}

#[derive(Deserialize)]
struct ShortenPayload {
    url: String,
}

async fn redirect(
    Path(key): Path<String>,
    State(state): State<SharedState>,
) -> Result<Redirect, StatusCode> {
    let db = &state.read().unwrap().db;

    if let Some(value) = db.get(&key) {
        Ok(Redirect::temporary(value))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// TODO: collision strategy
async fn shorten_url(
    State(state): State<SharedState>,
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
    State(state): State<SharedState>,
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    let db = &state.read().unwrap().db;
    // TODO: without cloning
    Ok(Json(db.clone()))
}

fn admin_routes() -> Router<SharedState> {
    async fn delete_all_keys(State(state): State<SharedState>) -> String {
        let count = state.write().unwrap().db.drain().count();
        count.to_string()
    }

    async fn remove_key(
        Path(key): Path<String>,
        State(state): State<SharedState>,
    ) -> Result<String, StatusCode> {
        if let Some(value) = state.write().unwrap().db.remove(&key) {
            Ok(value)
        } else {
            Err(StatusCode::NOT_FOUND)
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
