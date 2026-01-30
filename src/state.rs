use crate::config::Config;
use sqlx::postgres::PgPool;

pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}
