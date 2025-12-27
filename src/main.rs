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
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use url::Url;

const BASE_URL: &str = "tg.com";

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let admin_username =
        std::env::var("ADMIN_USERNAME").expect("Environment variable ADMIN_USERNAME");
    let admin_password =
        std::env::var("ADMIN_PASSWORD").expect("Environment variable ADMIN_PASSWORD");

    let shared_state = Arc::new(RwLock::new(AppState {
        db: HashMap::new(),
        admin_username,
        admin_password,
    }));

    let app = Router::new()
        .route("/{key}", get(redirect))
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

// TODO: handle unwraps for app state
type SharedState = Arc<RwLock<AppState>>;

#[derive(Default)]
struct AppState {
    // TODO: convert database to Postgres
    db: HashMap<String, String>,
    admin_username: String,
    admin_password: String,
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
    async fn delete_all_keys(
        TypedHeader(Authorization(creds)): TypedHeader<Authorization<Basic>>,
        State(state): State<SharedState>,
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
        State(state): State<SharedState>,
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
