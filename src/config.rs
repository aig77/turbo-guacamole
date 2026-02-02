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
    pub cache_url: String,
    pub admin_username: String,
    pub admin_password: String,
    pub code_rate_limit_config: RateLimitConfig,
    pub shorten_rate_limit_config: RateLimitConfig,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            service_host: get_env("SERVICE_HOST").expect("SERVICE_HOST must be set"),
            service_port: get_env("SERVICE_PORT").expect("SERVICE_PORT"),
            database_url: get_env("DATABASE_URL").expect("DATABASE_URL must be set"),
            cache_url: get_env("CACHE_URL").expect("CACHE_URL must be set"),
            admin_username: get_env("ADMIN_USERNAME").expect("ADMIN_USERNAME must be set"),
            admin_password: get_env("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set"),
            code_rate_limit_config: RateLimitConfig {
                requests_per_second: get_env("CODE_REQUESTS_PER_SECOND")
                    .expect("CODE_REQUESTS_PER_SECOND must be set"),
                burst_size: get_env("CODE_BURST_SIZE").expect("CODE_BURST_SIZE must be set"),
                cleanup_interval_secs: get_env("CODE_CLEANUP_INTERVAL_SECS")
                    .expect("CODE_CLEANUP_INTERVAL_SECS must be set"),
            },
            shorten_rate_limit_config: RateLimitConfig {
                requests_per_second: get_env("SHORTEN_REQUESTS_PER_SECOND")
                    .expect("SHORTEN_REQUESTS_PER_SECOND must be set"),
                burst_size: get_env("SHORTEN_BURST_SIZE").expect("SHORTEN_BURST_SIZE must be set"),
                cleanup_interval_secs: get_env("SHORTEN_CLEANUP_INTERVAL_SECS")
                    .expect("SHORTEN_CLEANUP_INTERVAL_SECS must be set"),
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
