use std::sync::Arc;

use moka::future::Cache;

use crate::{
    config::Config,
    error::VelesError,
    fetch::{FetchedPage, Fetcher},
    rate_limit::RateLimiter,
    search::{DuckDuckGoSearch, SearchResponse},
};

#[derive(Debug, Clone)]
pub struct AppState {
    search: DuckDuckGoSearch,
    fetcher: Fetcher,
    rate_limiter: Arc<RateLimiter>,
    search_cache: Cache<String, SearchResponse>,
    fetch_cache: Cache<String, FetchedPage>,
}

impl AppState {
    pub fn new(config: Config) -> Result<Self, VelesError> {
        let cache_ttl = config.cache_ttl;

        Ok(Self {
            search: DuckDuckGoSearch::new(&config)?,
            fetcher: Fetcher::new(&config)?,
            rate_limiter: Arc::new(RateLimiter::new(config.requests_per_second)),
            search_cache: Cache::builder().time_to_live(cache_ttl).build(),
            fetch_cache: Cache::builder().time_to_live(cache_ttl).build(),
        })
    }

    pub async fn search(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<SearchResponse, VelesError> {
        let key = format!("{query}\0{max_results}");
        if let Some(cached) = self.search_cache.get(&key).await {
            return Ok(cached);
        }

        let response = self
            .search
            .search(query, max_results, &self.rate_limiter)
            .await?;
        self.search_cache.insert(key, response.clone()).await;

        Ok(response)
    }

    pub async fn fetch(&self, url: &str) -> Result<FetchedPage, VelesError> {
        if let Some(cached) = self.fetch_cache.get(url).await {
            return Ok(cached);
        }

        let page = self.fetcher.fetch(url, &self.rate_limiter).await?;
        self.fetch_cache.insert(url.to_owned(), page.clone()).await;

        Ok(page)
    }
}
