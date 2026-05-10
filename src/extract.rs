use scraper::{Html, Selector};
use serde::Serialize;

use crate::fetch::FetchedPage;

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct ExtractedPage {
    pub url: String,
    pub final_url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical_url: Option<String>,
    pub text: String,
}

pub fn extract_page(page: &FetchedPage) -> ExtractedPage {
    let html = Html::parse_document(&page.text);

    let title = select_text(&html, "title")
        .or_else(|| select_attr(&html, "meta[property='og:title']", "content"));
    let description = select_attr(&html, "meta[name='description']", "content")
        .or_else(|| select_attr(&html, "meta[property='og:description']", "content"));
    let canonical_url = select_attr(&html, "link[rel='canonical']", "href");

    let markdown = html2md::parse_html(&page.text);
    let text = clean_text(&markdown);

    ExtractedPage {
        url: page.url.clone(),
        final_url: page.final_url.clone(),
        title,
        description,
        canonical_url,
        text,
    }
}

fn select_text(html: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    html.select(&selector)
        .next()
        .map(|node| clean_text(&node.text().collect::<Vec<_>>().join(" ")))
        .filter(|value| !value.is_empty())
}

fn select_attr(html: &Html, selector: &str, attr: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    html.select(&selector)
        .next()
        .and_then(|node| node.value().attr(attr))
        .map(clean_text)
        .filter(|value| !value.is_empty())
}

pub fn clean_text(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
