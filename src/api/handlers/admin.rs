use crate::{db::queries::urls, state::AppState};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Basic},
};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, error, info, instrument, warn};

pub fn admin_routes() -> Router<Arc<AppState>> {
    #[instrument(skip(state))]
    async fn list_codes(
        TypedHeader(Authorization(creds)): TypedHeader<Authorization<Basic>>,
        State(state): State<Arc<AppState>>,
    ) -> Result<Json<HashMap<String, String>>, StatusCode> {
        authenticate(
            creds.username(),
            creds.password(),
            &state.config.admin_username,
            &state.config.admin_password,
        )?;

        let urls: HashMap<String, String> = urls::list_all(&state.pg_pool)
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
            &state.config.admin_username,
            &state.config.admin_password,
        )?;

        let result = urls::delete_all(&state.pg_pool).await.map_err(|e| {
            error!("Database delete failed while removing all codes: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // also clear cache
        if let Ok(mut conn) = state.redis_pool.get().await {
            match redis::cmd("FLUSHDB")
                .arg("ASYNC")
                .query_async::<()>(&mut *conn)
                .await
            {
                Ok(_) => debug!("Also deleted from cache"),
                Err(_) => debug!("Failed to delete from cache"),
            }
        }

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
            &state.config.admin_username,
            &state.config.admin_password,
        )?;

        match urls::delete_code(&state.pg_pool, &code).await {
            Ok(Some(url)) => {
                info!("Code successfully deleted");
                // also delete from cache if available
                if let Ok(mut conn) = state.redis_pool.get().await {
                    match redis::cmd("DEL")
                        .arg(format!("short:{code}"))
                        .query_async::<()>(&mut *conn)
                        .await
                    {
                        Ok(_) => debug!("Also deleted from cache"),
                        Err(_) => debug!("Failed to delete from cache"),
                    }
                }
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
