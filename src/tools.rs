use serde::{Deserialize, Serialize};

use crate::{extract::ExtractedPage, fetch::FetchedPage, search::SearchResult};

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
pub struct WebResearchParams {
    pub query: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_fetch_top_n")]
    pub fetch_top_n: usize,
    #[serde(default = "default_max_chars")]
    pub max_chars_per_page: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WebSearchOutput {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WebFetchOutput {
    pub page: FetchedPage,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WebExtractOutput {
    pub page: ExtractedPage,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct WebResearchOutput {
    pub query: String,
    pub sources: Vec<ResearchSource>,
    pub note: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ResearchSource {
    pub title: Option<String>,
    pub url: String,
    pub search_snippet: String,
    pub excerpt: String,
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

fn default_max_results() -> usize {
    5
}

fn default_fetch_top_n() -> usize {
    3
}

fn default_max_chars() -> usize {
    12000
}
