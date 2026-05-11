use rmcp::ErrorData;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum VelesError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("blocked URL: {0}")]
    BlockedUrl(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("HTTP status {status} for {url}")]
    HttpStatus { url: String, status: u16 },

    #[error("response is too large: {size} bytes exceeds limit {limit} bytes")]
    ResponseTooLarge { size: u64, limit: u64 },

    #[error("search parsing failed: {0}")]
    SearchParse(String),

    #[error("browser rendering is disabled")]
    BrowserDisabled,

    #[error("browser rendering requires explicit user permission")]
    BrowserPermissionRequired,

    #[error("browser rendering failed: {0}")]
    Browser(String),
}

impl From<VelesError> for ErrorData {
    fn from(value: VelesError) -> Self {
        match value {
            VelesError::InvalidUrl(message) | VelesError::BlockedUrl(message) => {
                ErrorData::invalid_params(message, None)
            }
            other => {
                ErrorData::internal_error(other.to_string(), Some(json!({ "source": "veles" })))
            }
        }
    }
}
