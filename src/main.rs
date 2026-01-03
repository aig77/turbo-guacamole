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
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

struct AppState {
    pool: PgPool,
    base_url: String,
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

async fn shorten_url(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ShortenPayload>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // validate
    let parsed = Url::parse(&payload.url)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", e)))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Only http and https schemes are accepted".to_string(),
        ));
    }

    let code = encode(&payload.url);

    let shortened = shortened_url_from_code(&code, &state.base_url);

    // grab code and url for collision handling later
    let result: Result<Option<(String, String)>, _> =
        sqlx::query_as("SELECT code, url FROM urls WHERE code = $1")
            .bind(&code)
            .fetch_optional(&state.pool)
            .await;

    match result {
        Ok(Some((_, url))) => {
            // TODO: enhance collision strategy
            // check if result url is equal to the payload (the same url has been added before)
            // return status ok in this case
            // otherwise, we have to handle collision strategy
            if url == payload.url {
                Ok((StatusCode::OK, shortened))
            } else {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "code collision".to_string(),
                ))
            }
        }
        Ok(None) => {
            sqlx::query("INSERT INTO urls (code, url) VALUES ($1, $2)")
                .bind(&code)
                .bind(&payload.url)
                .execute(&state.pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok((StatusCode::CREATED, shortened))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

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

        let result = sqlx::query_as::<_, (String, String)>(
            "DELETE FROM urls where code = $1 RETURNING code, url",
        )
        .bind(&code)
        .fetch_optional(&state.pool)
        .await;

        match result {
            Ok(Some((code, url))) => Ok(format!("{}: {}", code, url)),
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

// TODO: shorten code to 5 or 6 at base and use collision to expand
fn encode(s: &str) -> String {
    let hash = Sha256::digest(s);
    let bytes = hash.as_slice();
    let eight_bytes: [u8; 8] = bytes[..8].try_into().unwrap();
    let number = u64::from_be_bytes(eight_bytes);
    format!("{:0>8}", base62::encode(number))
}

fn shortened_url_from_code(code: &str, base_url: &str) -> String {
    let mut shortened = String::from(base_url);
    shortened.push('/');
    shortened.push_str(code);
    shortened
}
