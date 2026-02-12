use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error(
        "Invalid rate limit format for {env_var}. Expected format: 'requests_per_second:burst_size:cleanup_interval_secs' (e.g., '20:30:60'), got: '{value}'"
    )]
    InvalidRateLimitFormat { env_var: String, value: String },

    #[error("Failed to parse rate limit value in {env_var}: {source}")]
    ParseError {
        env_var: String,
        #[source]
        source: std::num::ParseIntError,
    },
}

#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    pub requests_per_second: u64,
    pub burst_size: u32,
    pub cleanup_interval_secs: u64,
}

impl RateLimitConfig {
    fn from_env_or_default(env_var: &str, default: &str) -> Result<Self, ConfigError> {
        let value = std::env::var(env_var).unwrap_or_else(|_| default.to_string());
        Self::parse(&value, env_var)
    }

    fn parse(value: &str, env_var: &str) -> Result<Self, ConfigError> {
        let parts: Vec<&str> = value.split(':').collect();

        if parts.len() != 3 {
            return Err(ConfigError::InvalidRateLimitFormat {
                env_var: env_var.to_string(),
                value: value.to_string(),
            });
        }

        let requests_per_second = parts[0]
            .parse::<u64>()
            .map_err(|e| ConfigError::ParseError {
                env_var: env_var.to_string(),
                source: e,
            })?;

        let burst_size = parts[1]
            .parse::<u32>()
            .map_err(|e| ConfigError::ParseError {
                env_var: env_var.to_string(),
                source: e,
            })?;

        let cleanup_interval_secs =
            parts[2]
                .parse::<u64>()
                .map_err(|e| ConfigError::ParseError {
                    env_var: env_var.to_string(),
                    source: e,
                })?;

        Ok(Self {
            requests_per_second,
            burst_size,
            cleanup_interval_secs,
        })
    }
}

impl std::default::Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            requests_per_second: 5,
            burst_size: 10,
            cleanup_interval_secs: 60,
        }
    }
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
            service_port: get_env("SERVICE_PORT").expect("SERVICE_PORT must be set"),
            database_url: get_env("DATABASE_URL").expect("DATABASE_URL must be set"),
            stale_urls_days: get_env("STALE_URLS_DAYS").unwrap_or(90),
            cache_url: get_env("CACHE_URL").expect("CACHE_URL must be set"),
            redirect_rate_limit_config: RateLimitConfig::from_env_or_default(
                "REDIRECT_RATE_LIMIT",
                "20:30:60",
            )
            .expect("Failed to parse REDIRECT_RATE_LIMIT"),
            shorten_rate_limit_config: RateLimitConfig::from_env_or_default(
                "SHORTEN_RATE_LIMIT",
                "5:10:300",
            )
            .expect("Failed to parse SHORTEN_RATE_LIMIT"),
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
