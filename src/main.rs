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

const BASE_URL: &str = "tg.com";

#[tokio::main]
async fn main() {
    let shared_state = SharedState::default();

    let app = Router::new()
        // will eventually route, for now just print the value
        .route("/{key}", get(redirect))
        .route("/shorten", post(add_url))
        .nest("/admin", admin_routes())
        .with_state(Arc::clone(&shared_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    dbg!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

type SharedState = Arc<RwLock<AppState>>;

#[derive(Default)]
struct AppState {
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

async fn add_url(
    State(state): State<SharedState>,
    Json(payload): Json<ShortenPayload>,
) -> Result<String, StatusCode> {
    let key = encode(payload.url.as_str());
    let shortened = shortened_url_from_key(&key);
    state.write().unwrap().db.insert(key, payload.url);
    Ok(shortened)
}

fn admin_routes() -> Router<SharedState> {
    async fn list_keys(State(state): State<SharedState>) -> Result<String, StatusCode> {
        let db = &state.read().unwrap().db;

        Ok(db
            .keys()
            .map(|key| key.to_string())
            .collect::<Vec<String>>()
            .join("\n"))
    }

    async fn delete_all_keys(State(state): State<SharedState>) {
        state.write().unwrap().db.clear();
    }

    async fn remove_key(Path(key): Path<String>, State(state): State<SharedState>) {
        state.write().unwrap().db.remove(&key);
    }

    Router::new()
        .route("/keys", get(list_keys))
        .route("/delete/keys", delete(delete_all_keys))
        .route("/delete/keys/{key}", delete(remove_key))
}

fn encode(s: &str) -> String {
    let hash = Sha256::digest(s);
    let bytes = hash.as_slice();
    let eight_bytes: [u8; 8] = bytes[..8].try_into().unwrap();
    let number = u64::from_be_bytes(eight_bytes);
    base62::encode(number)
}

fn shortened_url_from_key(key: &str) -> String {
    let mut shortened = String::from(BASE_URL);
    shortened.push('/');
    shortened.push_str(key);
    shortened
}
