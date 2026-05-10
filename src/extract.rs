use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use url::Url;

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

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct ReadablePage {
    pub url: String,
    pub final_url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical_url: Option<String>,
    pub content_type: Option<String>,
    pub markdown: String,
    pub links: Vec<ReadableLink>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct ReadableLink {
    pub text: String,
    pub url: String,
}

pub fn extract_page(page: &FetchedPage) -> ExtractedPage {
    let html = Html::parse_document(&page.text);

    let metadata = page_metadata(&html);

    let markdown = html2md::parse_html(&page.text);
    let text = clean_text(&markdown);

    ExtractedPage {
        url: page.url.clone(),
        final_url: page.final_url.clone(),
        title: metadata.title,
        description: metadata.description,
        canonical_url: metadata.canonical_url,
        text,
    }
}

pub fn read_page(page: &FetchedPage, max_chars: usize) -> ReadablePage {
    let html = Html::parse_document(&page.text);
    let metadata = page_metadata(&html);
    let body = readable_body(&html).unwrap_or_else(|| clean_text(&html2md::parse_html(&page.text)));
    let markdown = build_markdown(
        metadata.title.as_deref(),
        metadata.description.as_deref(),
        &body,
    );
    let (markdown, truncated) = truncate_with_flag(&markdown, max_chars);
    let links = extract_links(&html, &page.final_url);

    ReadablePage {
        url: page.url.clone(),
        final_url: page.final_url.clone(),
        title: metadata.title,
        description: metadata.description,
        canonical_url: metadata.canonical_url,
        content_type: page.content_type.clone(),
        markdown,
        links,
        truncated,
    }
}

struct PageMetadata {
    title: Option<String>,
    description: Option<String>,
    canonical_url: Option<String>,
}

fn page_metadata(html: &Html) -> PageMetadata {
    PageMetadata {
        title: select_text(html, "title")
            .or_else(|| select_attr(html, "meta[property='og:title']", "content")),
        description: select_attr(html, "meta[name='description']", "content")
            .or_else(|| select_attr(html, "meta[property='og:description']", "content")),
        canonical_url: select_attr(html, "link[rel='canonical']", "href"),
    }
}

fn readable_body(html: &Html) -> Option<String> {
    let candidates = [
        "main",
        "article",
        "[role='main']",
        ".entry-content",
        ".post-content",
        ".article-content",
        ".content",
        "#content",
        "body",
    ];

    for candidate in candidates {
        let selector = Selector::parse(candidate).ok()?;
        for node in html.select(&selector) {
            let text = readable_blocks(node);
            if text.chars().count() >= 200 {
                return Some(text);
            }
        }
    }

    None
}

fn readable_blocks(root: ElementRef<'_>) -> String {
    let block_selector = Selector::parse("h1,h2,h3,h4,h5,h6,p,li,blockquote,pre,td,th").ok();
    let mut blocks = Vec::new();

    if let Some(selector) = block_selector {
        for block in root.select(&selector) {
            if should_skip_node(block) {
                continue;
            }

            let text = clean_inline_text(&block.text().collect::<Vec<_>>().join(" "));
            if text.is_empty() {
                continue;
            }

            let prefix = match block.value().name() {
                "h1" => "# ",
                "h2" => "## ",
                "h3" => "### ",
                "h4" => "#### ",
                "h5" => "##### ",
                "h6" => "###### ",
                "li" => "- ",
                _ => "",
            };

            blocks.push(format!("{prefix}{text}"));
        }
    }

    if blocks.is_empty() {
        clean_text(&root.text().collect::<Vec<_>>().join(" "))
    } else {
        blocks.join("\n\n")
    }
}

fn build_markdown(title: Option<&str>, description: Option<&str>, body: &str) -> String {
    let mut markdown = String::new();

    if let Some(title) = title {
        markdown.push_str("# ");
        markdown.push_str(title);
        markdown.push_str("\n\n");
    }

    if let Some(description) = description {
        markdown.push_str("> ");
        markdown.push_str(description);
        markdown.push_str("\n\n");
    }

    markdown.push_str(body);
    clean_text(&markdown)
}

fn extract_links(html: &Html, base_url: &str) -> Vec<ReadableLink> {
    let Ok(selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let base = Url::parse(base_url).ok();
    let mut links = Vec::new();

    for node in html.select(&selector) {
        if should_skip_node(node) {
            continue;
        }

        let text = clean_inline_text(&node.text().collect::<Vec<_>>().join(" "));
        let Some(href) = node.value().attr("href") else {
            continue;
        };
        if should_skip_href(href) {
            continue;
        }

        let url = match &base {
            Some(base) => base.join(href).ok().map(|url| url.to_string()),
            None => Url::parse(href).ok().map(|url| url.to_string()),
        };

        if let Some(url) = url
            && !text.is_empty()
            && links.iter().all(|link: &ReadableLink| link.url != url)
        {
            links.push(ReadableLink { text, url });
        }

        if links.len() >= 100 {
            break;
        }
    }

    links
}

fn should_skip_node(node: ElementRef<'_>) -> bool {
    node.ancestors()
        .filter_map(ElementRef::wrap)
        .any(is_noise_element)
}

fn is_noise_element(node: ElementRef<'_>) -> bool {
    let value = node.value();
    if matches!(
        value.name(),
        "script"
            | "style"
            | "noscript"
            | "template"
            | "svg"
            | "header"
            | "footer"
            | "nav"
            | "aside"
            | "form"
            | "button"
            | "iframe"
    ) {
        return true;
    }

    if value.attr("hidden").is_some() || value.attr("aria-hidden") == Some("true") {
        return true;
    }

    if value.attr("style").is_some_and(|style| {
        contains_any(
            &style.to_ascii_lowercase(),
            &["display:none", "visibility:hidden"],
        )
    }) {
        return true;
    }

    let marker = [value.attr("id"), value.attr("class"), value.attr("role")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    contains_any(
        &marker,
        &[
            "ad-",
            "ads",
            "advert",
            "banner",
            "breadcrumb",
            "cookie",
            "consent",
            "footer",
            "header",
            "menu",
            "modal",
            "newsletter",
            "nav",
            "popup",
            "promo",
            "share",
            "sidebar",
            "social",
            "subscribe",
        ],
    )
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn should_skip_href(href: &str) -> bool {
    let href = href.trim().to_ascii_lowercase();
    href.is_empty()
        || href.starts_with('#')
        || href.starts_with("javascript:")
        || href.starts_with("mailto:")
        || href.starts_with("tel:")
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

fn clean_inline_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_with_flag(value: &str, max_chars: usize) -> (String, bool) {
    let mut result = String::new();
    let mut chars = value.chars();

    for ch in chars.by_ref().take(max_chars) {
        result.push(ch);
    }

    let truncated = chars.next().is_some();
    (result, truncated)
}

#[cfg(test)]
mod tests {
    use super::{FetchedPage, read_page};

    #[test]
    fn read_page_prefers_article_content() {
        let page = FetchedPage {
            url: "https://example.com".into(),
            final_url: "https://example.com".into(),
            status: 200,
            content_type: Some("text/html".into()),
            text: r#"
                <html>
                  <head><title>Example</title><meta name="description" content="Description"></head>
                  <body>
                    <nav>Navigation</nav>
                    <article>
                      <h1>Main Heading</h1>
                      <p>This is a long paragraph with useful content for the readable page extractor.</p>
                      <p>It should be preferred over the navigation and other page chrome.</p>
                      <p>Additional text makes the article sufficiently long for the candidate selector.</p>
                      <a href="/next">Next page</a>
                    </article>
                  </body>
                </html>
            "#
            .into(),
        };

        let readable = read_page(&page, 2000);

        assert!(readable.markdown.contains("# Example"));
        assert!(readable.markdown.contains("# Main Heading"));
        assert!(readable.markdown.contains("useful content"));
        assert_eq!(readable.links[0].url, "https://example.com/next");
        assert!(!readable.truncated);
    }

    #[test]
    fn read_page_ignores_common_noise_when_using_body() {
        let page = FetchedPage {
            url: "https://example.com".into(),
            final_url: "https://example.com".into(),
            status: 200,
            content_type: Some("text/html".into()),
            text: r#"
                <html>
                  <head><title>Example</title></head>
                  <body>
                    <nav><p>Navigation should not appear.</p><a href="/menu">Menu</a></nav>
                    <div class="cookie-banner"><p>Accept cookies should not appear.</p></div>
                    <p>Main body content starts here and contains useful information for the reader.</p>
                    <p>More useful information keeps the body selector above the minimum threshold.</p>
                    <p>This final paragraph adds enough real page text to make the fallback readable.</p>
                    <footer><p>Footer should not appear.</p></footer>
                  </body>
                </html>
            "#
            .into(),
        };

        let readable = read_page(&page, 2000);

        assert!(readable.markdown.contains("Main body content"));
        assert!(!readable.markdown.contains("Navigation should not appear"));
        assert!(
            !readable
                .markdown
                .contains("Accept cookies should not appear")
        );
        assert!(!readable.markdown.contains("Footer should not appear"));
        assert!(readable.links.is_empty());
    }
}
