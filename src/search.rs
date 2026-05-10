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
    pub warnings: Vec<String>,
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
        let mut response = self.search_once(query, max_results, rate_limiter).await?;

        if response.results.is_empty()
            && query.contains('"')
            && let Some(simplified_query) = simplify_query(query)
        {
            let mut fallback = self
                .search_once(&simplified_query, max_results, rate_limiter)
                .await?;
            fallback.query = query.to_owned();
            fallback.warnings.insert(
                0,
                format!("simplified_query_used: no results found for exact query; retried as: {simplified_query}"),
            );
            response = fallback;
        }

        Ok(response)
    }

    async fn search_once(
        &self,
        query: &str,
        max_results: usize,
        rate_limiter: &RateLimiter,
    ) -> Result<SearchResponse, VelesError> {
        let mut warnings = Vec::new();
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
            warnings.push(format!(
                "DuckDuckGo HTML endpoint returned HTTP status {}",
                status.as_u16()
            ));

            return Ok(SearchResponse {
                query: query.to_owned(),
                results: Vec::new(),
                warnings,
            });
        }

        let html = response.text().await?;
        if is_probable_block_page(&html) {
            warnings.push(
                "blocked_by_duckduckgo: DuckDuckGo HTML endpoint appears to have returned an anti-bot or challenge page"
                    .into(),
            );
        }
        let mut results = parse_ddg_html(&html)?;

        if results.is_empty() {
            warnings.push(
                "lite_fallback_used: DuckDuckGo HTML endpoint returned no parseable results; retried with Lite endpoint"
                    .into(),
            );
            results = self.search_lite(query, rate_limiter, &mut warnings).await?;
        }

        if results.is_empty() {
            warnings.push("no_results: no parseable DuckDuckGo results found".into());
        }

        results.truncate(max_results);

        Ok(SearchResponse {
            query: query.to_owned(),
            results,
            warnings,
        })
    }

    async fn search_lite(
        &self,
        query: &str,
        rate_limiter: &RateLimiter,
        warnings: &mut Vec<String>,
    ) -> Result<Vec<SearchResult>, VelesError> {
        rate_limiter.wait().await;

        let response = self
            .client
            .get("https://lite.duckduckgo.com/lite/")
            .query(&[
                ("q", query),
                ("kl", self.region.as_str()),
                ("kp", self.safe_search_kp.as_str()),
            ])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            warnings.push(format!(
                "DuckDuckGo Lite endpoint returned HTTP status {}",
                status.as_u16()
            ));
            return Ok(Vec::new());
        }

        let html = response.text().await?;
        if is_probable_block_page(&html) {
            warnings.push(
                "blocked_by_duckduckgo: DuckDuckGo Lite endpoint appears to have returned an anti-bot or challenge page"
                    .into(),
            );
        }
        parse_ddg_lite_html(&html)
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

    Ok(results)
}

fn parse_ddg_lite_html(html: &str) -> Result<Vec<SearchResult>, VelesError> {
    let document = Html::parse_document(html);
    let row_selector = parse_selector("tr")?;
    let link_selector = parse_selector("a[href]")?;
    let mut results = Vec::new();
    let rows = document.select(&row_selector).collect::<Vec<_>>();

    for (index, row) in rows.iter().enumerate() {
        let Some((title, url)) = lite_result_from_node(*row, &link_selector) else {
            continue;
        };
        if results
            .iter()
            .any(|result: &SearchResult| result.url == url)
        {
            continue;
        }

        let snippet = rows
            .iter()
            .skip(index + 1)
            .take(3)
            .map(|row| clean_inline_text(&row.text().collect::<Vec<_>>().join(" ")))
            .find(|text| is_useful_snippet(text, &title))
            .unwrap_or_default();

        results.push(SearchResult {
            title,
            url,
            snippet,
            source: "duckduckgo_lite".into(),
        });

        if results.len() >= 20 {
            return Ok(results);
        }
    }

    for link in document.select(&link_selector) {
        let title = clean_inline_text(&link.text().collect::<Vec<_>>().join(" "));
        let Some(raw_href) = link.value().attr("href") else {
            continue;
        };
        let Some(url) = normalize_ddg_url(raw_href) else {
            continue;
        };
        if title.is_empty()
            || should_skip_lite_result(&url)
            || results
                .iter()
                .any(|result: &SearchResult| result.url == url)
        {
            continue;
        }

        results.push(SearchResult {
            title,
            url,
            snippet: String::new(),
            source: "duckduckgo_lite".into(),
        });

        if results.len() >= 20 {
            break;
        }
    }

    Ok(results)
}

fn lite_result_from_node(
    node: scraper::ElementRef<'_>,
    link_selector: &Selector,
) -> Option<(String, String)> {
    for link in node.select(link_selector) {
        let title = clean_inline_text(&link.text().collect::<Vec<_>>().join(" "));
        let raw_href = link.value().attr("href")?;
        let url = normalize_ddg_url(raw_href)?;

        if !title.is_empty() && !should_skip_lite_result(&url) {
            return Some((title, url));
        }
    }

    None
}

fn clean_inline_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_useful_snippet(value: &str, title: &str) -> bool {
    !value.is_empty()
        && value != title
        && value.chars().count() >= 20
        && !value.contains("http://")
        && !value.contains("https://")
}

fn should_skip_lite_result(url: &str) -> bool {
    Url::parse(url)
        .ok()
        .and_then(|url| url.domain().map(str::to_owned))
        .is_some_and(|domain| domain.ends_with("duckduckgo.com"))
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

fn simplify_query(query: &str) -> Option<String> {
    let simplified = query.replace('"', " ");
    let simplified = simplified.split_whitespace().collect::<Vec<_>>().join(" ");

    if simplified == query || simplified.is_empty() {
        None
    } else {
        Some(simplified)
    }
}

fn is_probable_block_page(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    [
        "captcha",
        "challenge",
        "unusual traffic",
        "verify you are human",
        "automated requests",
        "anomaly detected",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::{is_probable_block_page, parse_ddg_html, parse_ddg_lite_html};

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

    #[test]
    fn empty_duckduckgo_html_is_not_an_error() {
        let results =
            parse_ddg_html("<html><body>No results</body></html>").expect("parse results");

        assert!(results.is_empty());
    }

    #[test]
    fn parses_duckduckgo_lite_result() {
        let html = r#"
            <html>
              <body>
                <table>
                  <tr>
                    <td><a href="/l/?uddg=https%3A%2F%2Fexample.com%2Frestaurant">Example Restaurant</a></td>
                  </tr>
                  <tr>
                    <td>Useful restaurant snippet from the Lite result page.</td>
                  </tr>
                </table>
              </body>
            </html>
        "#;

        let results = parse_ddg_lite_html(html).expect("parse lite results");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example Restaurant");
        assert_eq!(results[0].url, "https://example.com/restaurant");
        assert_eq!(
            results[0].snippet,
            "Useful restaurant snippet from the Lite result page."
        );
        assert_eq!(results[0].source, "duckduckgo_lite");
    }

    #[test]
    fn detects_probable_block_page() {
        assert!(is_probable_block_page(
            "<html><title>Captcha</title>Verify you are human</html>"
        ));
        assert!(!is_probable_block_page(
            "<html><body>regular search results</body></html>"
        ));
    }
}
