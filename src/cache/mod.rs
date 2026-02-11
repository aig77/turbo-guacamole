use tracing::debug;

pub type RedisPool = bb8::Pool<redis::Client>;

pub async fn setup_cache(url: &str) -> Result<RedisPool, redis::RedisError> {
    let client = redis::Client::open(url)?;
    bb8::Pool::builder().build(client).await
}

pub async fn add_to_cache(pool: &RedisPool, code: &str, url: &str) {
    if let Ok(mut conn) = pool.get().await {
        let _ = redis::cmd("SET")
            .arg(format!("short:{code}"))
            .arg(url)
            .arg("EX")
            .arg(3600)
            .query_async::<()>(&mut *conn)
            .await;

        debug!("Inserted into cache");
    } else {
        debug!("Failed to connect to redis pool when inserting");
    }
}
