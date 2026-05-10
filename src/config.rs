use std::{env, time::Duration};

use crate::error::VelesError;

#[derive(Debug, Clone)]
pub struct Config {
    pub requests_per_second: u32,
    pub cache_ttl: Duration,
    pub request_timeout: Duration,
    pub max_page_bytes: u64,
    pub ddg_region: String,
    pub safe_search: SafeSearch,
    pub user_agent: String,
}

impl Config {
    pub fn from_env() -> Result<Self, VelesError> {
        let requests_per_second = read_env("VELES_REQUESTS_PER_SECOND", 1)?;
        if requests_per_second == 0 {
            return Err(VelesError::Config(
                "VELES_REQUESTS_PER_SECOND must be greater than 0".into(),
            ));
        }

        Ok(Self {
            requests_per_second,
            cache_ttl: Duration::from_secs(read_env("VELES_CACHE_TTL_SECONDS", 3600)?),
            request_timeout: Duration::from_millis(read_env("VELES_REQUEST_TIMEOUT_MS", 15000)?),
            max_page_bytes: read_env("VELES_MAX_PAGE_BYTES", 2_000_000)?,
            ddg_region: env::var("VELES_DDG_REGION").unwrap_or_else(|_| "wt-wt".into()),
            safe_search: SafeSearch::from_env_value(
                &env::var("VELES_SAFESEARCH").unwrap_or_else(|_| "moderate".into()),
            )?,
            user_agent: env::var("VELES_USER_AGENT")
                .unwrap_or_else(|_| "Veles/0.1 local MCP server".into()),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SafeSearch {
    Strict,
    Moderate,
    Off,
}

impl SafeSearch {
    pub fn from_env_value(value: &str) -> Result<Self, VelesError> {
        match value.to_ascii_lowercase().as_str() {
            "strict" => Ok(Self::Strict),
            "moderate" => Ok(Self::Moderate),
            "off" => Ok(Self::Off),
            other => Err(VelesError::Config(format!(
                "unsupported VELES_SAFESEARCH value: {other}"
            ))),
        }
    }

    pub fn ddg_kp(self) -> &'static str {
        match self {
            Self::Strict => "1",
            Self::Moderate => "-1",
            Self::Off => "-2",
        }
    }
}

fn read_env<T>(name: &str, default: T) -> Result<T, VelesError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|err| VelesError::Config(format!("invalid {name}: {err}"))),
        Err(_) => Ok(default),
    }
}
