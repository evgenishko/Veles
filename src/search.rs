use scraper::{Html, Selector};
use serde::Serialize;
use url::Url;

use crate::{config::Config, error::VelesError, extract::clean_text, rate_limit::RateLimiter};

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone)]
pub struct DuckDuckGoSearch {
    client: reqwest::Client,
    region: String,
    safe_search_kp: String,
}

impl DuckDuckGoSearch {
    pub fn new(config: &Config) -> Result<Self, VelesError> {
        let client = reqwest::Client::builder()
            .user_agent(config.user_agent.clone())
            .timeout(config.request_timeout)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;

        Ok(Self {
            client,
            region: config.ddg_region.clone(),
            safe_search_kp: config.safe_search.ddg_kp().into(),
        })
    }

    pub async fn search(
        &self,
        query: &str,
        max_results: usize,
        rate_limiter: &RateLimiter,
    ) -> Result<SearchResponse, VelesError> {
        rate_limiter.wait().await;

        let response = self
            .client
            .get("https://html.duckduckgo.com/html/")
            .query(&[
                ("q", query),
                ("kl", self.region.as_str()),
                ("kp", self.safe_search_kp.as_str()),
            ])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return Err(VelesError::HttpStatus {
                url: response.url().to_string(),
                status: status.as_u16(),
            });
        }

        let html = response.text().await?;
        let mut results = parse_ddg_html(&html)?;
        results.truncate(max_results);

        Ok(SearchResponse {
            query: query.to_owned(),
            results,
        })
    }
}

fn parse_ddg_html(html: &str) -> Result<Vec<SearchResult>, VelesError> {
    let document = Html::parse_document(html);
    let result_selector = parse_selector(".result")?;
    let title_selector = parse_selector(".result__a")?;
    let snippet_selector = parse_selector(".result__snippet")?;

    let mut results = Vec::new();

    for result in document.select(&result_selector) {
        let Some(link) = result.select(&title_selector).next() else {
            continue;
        };

        let title = clean_text(&link.text().collect::<Vec<_>>().join(" "));
        let Some(raw_href) = link.value().attr("href") else {
            continue;
        };
        let Some(url) = normalize_ddg_url(raw_href) else {
            continue;
        };

        let snippet = result
            .select(&snippet_selector)
            .next()
            .map(|node| clean_text(&node.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_default();

        if !title.is_empty() && !url.is_empty() {
            results.push(SearchResult {
                title,
                url,
                snippet,
                source: "duckduckgo".into(),
            });
        }
    }

    if results.is_empty() {
        return Err(VelesError::SearchParse(
            "DuckDuckGo returned no parseable search results".into(),
        ));
    }

    Ok(results)
}

fn parse_selector(selector: &str) -> Result<Selector, VelesError> {
    Selector::parse(selector).map_err(|err| VelesError::SearchParse(err.to_string()))
}

fn normalize_ddg_url(raw_href: &str) -> Option<String> {
    let href = if raw_href.starts_with("//") {
        format!("https:{raw_href}")
    } else if raw_href.starts_with('/') {
        format!("https://duckduckgo.com{raw_href}")
    } else {
        raw_href.to_owned()
    };

    let parsed = Url::parse(&href).ok()?;
    if parsed.domain() == Some("duckduckgo.com") && parsed.path().starts_with("/l/") {
        for (key, value) in parsed.query_pairs() {
            if key == "uddg" {
                return Some(value.into_owned());
            }
        }
    }

    Some(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_ddg_html;

    #[test]
    fn parses_duckduckgo_html_result() {
        let html = r#"
            <div class="result">
              <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fdocs">Example Docs</a>
              <a class="result__snippet">Useful docs snippet.</a>
            </div>
        "#;

        let results = parse_ddg_html(html).expect("parse results");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example Docs");
        assert_eq!(results[0].url, "https://example.com/docs");
        assert_eq!(results[0].snippet, "Useful docs snippet.");
    }
}
