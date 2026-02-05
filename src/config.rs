use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    pub requests_per_second: u64,
    pub burst_size: u32,
    pub cleanup_interval_secs: u64,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub service_host: String,
    pub service_port: String,
    pub database_url: String,
    pub stale_urls_days: i32,
    pub cache_url: String,
    pub redirect_rate_limit_config: RateLimitConfig,
    pub shorten_rate_limit_config: RateLimitConfig,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            service_host: get_env("SERVICE_HOST").expect("SERVICE_HOST must be set"),
            service_port: get_env("SERVICE_PORT").expect("SERVICE_PORT"),
            database_url: get_env("DATABASE_URL").expect("DATABASE_URL must be set"),
            stale_urls_days: get_env("STALE_URLS_DAYS").unwrap_or(90),
            cache_url: get_env("CACHE_URL").expect("CACHE_URL must be set"),
            redirect_rate_limit_config: RateLimitConfig {
                requests_per_second: get_env("REDIRECT_REQUESTS_PER_SECOND").unwrap_or(2),
                burst_size: get_env("REDIRECT_BURST_SIZE").unwrap_or(5),
                cleanup_interval_secs: get_env("REDIRECT_CLEANUP_INTERVAL_SECS").unwrap_or(60),
            },
            shorten_rate_limit_config: RateLimitConfig {
                requests_per_second: get_env("SHORTEN_REQUESTS_PER_SECOND").unwrap_or(12),
                burst_size: get_env("SHORTEN_BURST_SIZE").unwrap_or(5),
                cleanup_interval_secs: get_env("SHORTEN_CLEANUP_INTERVAL_SECS").unwrap_or(300),
            },
        }
    }
}

fn get_env<T: FromStr>(key: &str) -> Option<T>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(val) => Some(
            val.parse::<T>()
                .unwrap_or_else(|e| panic!("Failed to parse {}: {}", key, e)),
        ),
        Err(_) => None,
    }
}
