use crate::{cache::RedisPool, config::Config};
use sqlx::postgres::PgPool;

pub struct AppState {
    pub pg_pool: PgPool,
    pub redis_pool: RedisPool,
    pub config: Config,
}
