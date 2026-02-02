use redis::AsyncCommands;

pub type RedisPool = bb8::Pool<redis::Client>;

pub async fn setup_cache(url: &str) -> RedisPool {
    let client = redis::Client::open(url).unwrap();
    let pool = bb8::Pool::builder().build(client).await.unwrap();
    {
        // ping the database before starting
        let mut conn = pool.get().await.unwrap();
        conn.set::<&str, &str, ()>("foo", "bar").await.unwrap();
        let result: String = conn.get("foo").await.unwrap();
        assert_eq!(result, "bar");
    }
    tracing::debug!("successfully connected to redis and pinged it");
    pool
}
