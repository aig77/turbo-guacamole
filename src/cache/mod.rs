pub type RedisPool = bb8::Pool<redis::Client>;

pub async fn setup_cache(url: &str) -> Result<RedisPool, redis::RedisError> {
    let client = redis::Client::open(url)?;
    bb8::Pool::builder().build(client).await
}
