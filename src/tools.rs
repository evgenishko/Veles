use serde::{Deserialize, Serialize};

use crate::{
    extract::{ExtractedPage, ReadablePage},
    fetch::FetchedPage,
    search::SearchResult,
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WebSearchParams {
    pub query: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WebFetchParams {
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WebExtractParams {
    pub url: String,
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WebReadParams {
    pub url: String,
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WebResearchParams {
    pub query: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_fetch_top_n")]
    pub fetch_top_n: usize,
    #[serde(default = "default_max_chars")]
    pub max_chars_per_page: usize,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct CurrentDateTimeParams {}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WebSearchOutput {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WebFetchOutput {
    pub ok: bool,
    pub page: FetchedPage,
    pub error: Option<ToolIssue>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WebExtractOutput {
    pub ok: bool,
    pub page: ExtractedPage,
    pub error: Option<ToolIssue>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WebReadOutput {
    pub ok: bool,
    pub page: ReadablePage,
    pub error: Option<ToolIssue>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WebResearchOutput {
    pub query: String,
    pub sources: Vec<ResearchSource>,
    pub warnings: Vec<String>,
    pub note: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ResearchSource {
    pub ok: bool,
    pub title: Option<String>,
    pub url: String,
    pub search_snippet: String,
    pub excerpt: String,
    pub error: Option<ToolIssue>,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct ToolIssue {
    pub kind: String,
    pub message: String,
    pub status: Option<i32>,
    pub url: Option<String>,
}

impl ToolIssue {
    pub fn http_status(status: i32, url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            kind: "http_status".into(),
            message: format!("Remote site returned HTTP status {status}"),
            status: Some(status),
            url: Some(url),
        }
    }

    pub fn fetch_failed(message: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            kind: "fetch_failed".into(),
            message: message.into(),
            status: None,
            url: Some(url.into()),
        }
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CurrentDateTimeOutput {
    pub local_time: String,
    pub utc_time: String,
    pub unix_timestamp: i64,
    pub timezone_offset: String,
}

pub fn clamp_max_results(value: usize) -> usize {
    value.clamp(1, 10)
}

pub fn clamp_max_chars(value: usize) -> usize {
    value.clamp(500, 20000)
}

pub fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut result = String::new();
    for ch in value.chars().take(max_chars) {
        result.push(ch);
    }
    result
}

pub fn fetch_issue(page: &FetchedPage) -> Option<ToolIssue> {
    if page.is_success() {
        None
    } else {
        Some(ToolIssue::http_status(page.status, page.final_url.clone()))
    }
}

fn default_max_results() -> usize {
    5
}

fn default_fetch_top_n() -> usize {
    3
}

fn default_max_chars() -> usize {
    12000
}
