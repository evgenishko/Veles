# Veles

Veles is a local Rust-based MCP server that gives local LLMs controlled access to web search and page extraction through DuckDuckGo.

## What Is Veles

Veles is designed for local models running on your computer. It exposes a small set of MCP tools for searching the web, fetching pages, extracting readable text, and collecting source excerpts for research-style prompts.

The first MVP is intentionally conservative: it does not automate a browser, does not require API keys, and uses DuckDuckGo through unofficial HTML parsing.

## Features

- Local MCP server over stdio.
- Rust implementation.
- DuckDuckGo-only web search.
- Unofficial DuckDuckGo HTML parsing.
- Public HTTP/HTTPS page fetching.
- Markdown-like page extraction.
- Small research workflow: search, fetch top pages, return excerpts and URLs.
- Global outbound HTTP rate limit: 1 request per second by default.
- In-memory cache for search, fetch, and extraction results.

## Current Limitations

- Browser automation is not included in the MVP.
- JavaScript-heavy pages may extract poorly without a browser.
- DuckDuckGo parsing is unofficial and can break if DuckDuckGo changes its HTML.
- DuckDuckGo may still return 429, CAPTCHA, or temporary blocks under automated load.
- Veles does not synthesize final answers itself; it returns structured sources for the LLM.

## Safety Defaults

- Maximum 1 outbound HTTP request per second by default.
- Only `http://` and `https://` URLs are allowed.
- `localhost`, local IPs, private IPs, link-local IPs, and unsupported schemes are blocked by default.
- Redirects are limited.
- Response size is limited.
- Web page content should always be treated as untrusted input.

## Installation

Install Rust with rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Reload your shell or run:

```bash
. "$HOME/.cargo/env"
```

## Build From Source

Clone the repository and build the binary:

```bash
cargo build --release
```

The binary will be available at:

```text
target/release/veles
```

## Usage With OpenCode

Add Veles to your OpenCode config:

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "veles": {
      "type": "local",
      "command": ["/absolute/path/to/veles", "--stdio"],
      "enabled": true,
      "timeout": 10000,
      "environment": {
        "VELES_REQUESTS_PER_SECOND": "1",
        "VELES_CACHE_TTL_SECONDS": "3600",
        "VELES_DDG_REGION": "wt-wt",
        "VELES_SAFESEARCH": "moderate"
      }
    }
  }
}
```

Use the absolute path to `target/release/veles` or to any installed copy of the binary.

## Usage With LM Studio

LM Studio MCP configuration may vary by version. Add Veles as a local stdio MCP server and point the command to the compiled binary:

```jsonc
{
  "mcpServers": {
    "veles": {
      "command": "/absolute/path/to/veles",
      "args": ["--stdio"],
      "env": {
        "VELES_REQUESTS_PER_SECOND": "1",
        "VELES_CACHE_TTL_SECONDS": "3600"
      }
    }
  }
}
```

If your LM Studio version uses a different UI for MCP registration, choose a local stdio server and use the same command and arguments.

## Configuration

Veles is configured with environment variables:

| Variable | Default | Description |
| --- | --- | --- |
| `VELES_REQUESTS_PER_SECOND` | `1` | Global outbound HTTP request rate. |
| `VELES_CACHE_TTL_SECONDS` | `3600` | In-memory cache TTL. |
| `VELES_REQUEST_TIMEOUT_MS` | `15000` | HTTP request timeout. |
| `VELES_MAX_PAGE_BYTES` | `2000000` | Maximum response size. |
| `VELES_DDG_REGION` | `wt-wt` | DuckDuckGo region parameter. |
| `VELES_SAFESEARCH` | `moderate` | `strict`, `moderate`, or `off`. |
| `VELES_USER_AGENT` | `Veles/0.1 local MCP server` | HTTP user agent. |

## Available MCP Tools

`current_datetime`

Returns the current local and UTC date/time from the system clock without using the internet.

`web_search`

Searches DuckDuckGo and returns result titles, URLs, and snippets.

`web_fetch`

Fetches a public HTTP/HTTPS page and returns text plus response metadata.

`web_extract`

Fetches a page and extracts readable Markdown-like text and metadata.

`web_read`

Reads a public HTTP/HTTPS page and returns cleaner LLM-friendly markdown with metadata, links, and a truncation flag. This is the high-level page-reading tool closest to a built-in web fetcher.

`web_research`

Runs a small workflow: DuckDuckGo search, fetch top results, extract text, and return source excerpts.

## DuckDuckGo Notes

Veles uses unofficial DuckDuckGo HTML parsing. This requires no API key, but it is not a stable public API. It is suitable for local personal use, not high-volume production search.

If DuckDuckGo changes its HTML or blocks automated traffic, `web_search` may fail until the parser is updated.

## Security Notes

Web content is untrusted input. A fetched page can contain prompt injection text that tries to influence the model. Treat returned page text as data, not instructions.

Veles blocks obvious local/private targets by default, but this is not a complete sandbox. Do not expose Veles to untrusted remote users.

## License

Veles is licensed under the MIT License.

## Roadmap

- Improve DuckDuckGo Lite fallback parsing.
- Add disk cache option.
- Add optional SearXNG backend.
- Add Firefox/browser backend for JavaScript-heavy pages.
- Add more robust readable-content extraction.
- Add package/install scripts for common MCP clients.
