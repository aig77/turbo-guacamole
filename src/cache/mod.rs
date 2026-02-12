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

pub async fn get_stats(pool: &RedisPool) -> Option<(i64, i64)> {
    if let Ok(mut conn) = pool.get().await {
        let result = redis::cmd("GET")
            .arg("stats:global")
            .query_async(&mut *conn)
            .await;

        debug!("Inserted into cache");
        result.ok()
    } else {
        debug!("Failed to connect to redis pool when inserting");
        None
    }
}

pub async fn set_stats(pool: &RedisPool, total_urls: i64, total_clicks: i64, ttl_seconds: u64) {
    if let Ok(mut conn) = pool.get().await {
        let _ = redis::cmd("SET")
            .arg("stats:global")
            .arg(format!("{},{}", total_urls, total_clicks))
            .arg("EX")
            .arg(ttl_seconds)
            .query_async::<()>(&mut *conn)
            .await;

        debug!("Inserted into cache");
    } else {
        debug!("Failed to connect to redis pool when inserting");
    }
}
