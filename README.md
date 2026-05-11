# Veles

Veles is a local Rust-based MCP server that gives local LLMs controlled access to web search, page extraction, and opt-in browser rendering through DuckDuckGo and Firefox.

## What Is Veles

Veles is designed for local models running on your computer. It exposes a small set of MCP tools for searching the web, fetching pages, extracting readable text, and collecting source excerpts for research-style prompts.

The first MVP is intentionally conservative: it does not require API keys, uses DuckDuckGo through unofficial HTML parsing, and keeps browser rendering disabled until explicitly enabled.

## Features

- Local MCP server over stdio.
- Rust implementation.
- DuckDuckGo-only web search.
- Unofficial DuckDuckGo HTML parsing.
- Public HTTP/HTTPS page fetching.
- Markdown-like page extraction.
- Optional Firefox rendering for JavaScript-heavy pages.
- Small research workflow: search, fetch top pages, return excerpts and URLs.
- Global outbound HTTP rate limit: 1 request per second by default.
- In-memory cache for search and fetch results.

## Current Limitations

- Browser rendering requires Firefox, geckodriver, and `VELES_BROWSER_ENABLED=true`.
- Rendered pages do not expose reliable HTTP status codes through WebDriver.
- DuckDuckGo parsing is unofficial and can break if DuckDuckGo changes its HTML.
- DuckDuckGo may still return 429, CAPTCHA, or temporary blocks under automated load.
- Veles does not synthesize final answers itself; it returns structured sources for the LLM.

## Safety Defaults

- Maximum 1 outbound HTTP request per second by default.
- Only `http://` and `https://` URLs are allowed.
- `localhost`, local IPs, private IPs, link-local IPs, and unsupported schemes are blocked by default.
- Redirects are limited.
- Response size is limited.
- Browser rendering is disabled by default and requires explicit first-call consent with `allow_browser=true`.
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

The installer builds the release binary and writes an OpenCode MCP entry with `enabled: false`:

```bash
./scripts/install-opencode.sh
```

You can override the config path with `OPENCODE_CONFIG=/path/to/opencode.json ./scripts/install-opencode.sh`.

Manual configuration:

Add Veles to your OpenCode config:

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "veles": {
      "type": "local",
      "command": ["/absolute/path/to/veles", "--stdio"],
      "enabled": false,
      "timeout": 120000,
      "environment": {
        "VELES_REQUESTS_PER_SECOND": "1",
        "VELES_CACHE_TTL_SECONDS": "3600",
        "VELES_DDG_REGION": "wt-wt",
        "VELES_SAFESEARCH": "moderate",
        "VELES_BROWSER_ENABLED": "false",
        "VELES_BROWSER_DRIVER": "geckodriver",
        "VELES_BROWSER_HEADLESS": "true",
        "VELES_BROWSER_PAGE_TIMEOUT_MS": "90000",
        "VELES_BROWSER_SETTLE_MS": "2000"
      }
    }
  }
}
```

Use the absolute path to `target/release/veles` or to any installed copy of the binary.

Set `enabled` to `true` when you want OpenCode to start Veles.

## Usage With LM Studio

In LM Studio, open the `Program` tab, choose `Install`, then choose `Edit mcp.json`. Add Veles as a local stdio MCP server and point `command` to the compiled binary:

```jsonc
{
  "mcpServers": {
    "veles": {
      "command": "/absolute/path/to/veles",
      "args": ["--stdio"],
      "env": {
        "VELES_REQUESTS_PER_SECOND": "1",
        "VELES_CACHE_TTL_SECONDS": "3600",
        "VELES_DDG_REGION": "wt-wt",
        "VELES_SAFESEARCH": "moderate"
      }
    }
  }
}
```

If your LM Studio version uses a different UI for MCP registration, choose a local stdio server and use the same command, arguments, and environment variables.

Recommended LM Studio settings:

- Server name: `veles`.
- Command: absolute path to `target/release/veles`.
- Arguments: `--stdio`.
- Environment: keep `VELES_REQUESTS_PER_SECOND=1` unless you intentionally want a higher global request rate.

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
| `VELES_USER_AGENT` | `Veles/0.5 local MCP server` | HTTP user agent. |
| `VELES_BROWSER_ENABLED` | `false` | Enable Firefox/geckodriver rendering for `web_read_rendered`. |
| `VELES_BROWSER_DRIVER` | `geckodriver` | Browser WebDriver executable path or command. |
| `VELES_FIREFOX_BINARY` | unset | Optional Firefox binary path. |
| `VELES_BROWSER_HEADLESS` | `true` | Run Firefox headless by default. |
| `VELES_BROWSER_PAGE_TIMEOUT_MS` | `90000` | Browser page load and startup timeout. |
| `VELES_BROWSER_SETTLE_MS` | `2000` | Extra wait after page load before reading rendered DOM. |

## Firefox Rendering

`web_read_rendered` is for JavaScript-heavy pages that do not expose useful content through plain HTTP fetching. It uses Firefox through geckodriver, creates a temporary private profile for each call, reads the rendered DOM, then closes the browser session and geckodriver process.

Requirements:

- Install Firefox.
- Install `geckodriver` and ensure it is available in `PATH`, or set `VELES_BROWSER_DRIVER` to its absolute path.
- Set `VELES_BROWSER_ENABLED=true`.

The first browser call in each Veles process requires explicit permission. A call without `allow_browser=true` returns `needs_browser_permission: true` and does not launch Firefox. After the user approves, retry the same tool call with `allow_browser=true`; Veles remembers that consent in memory until the MCP server restarts.

Firefox runs headless by default. Set `VELES_BROWSER_HEADLESS=false` or pass `headless: false` to `web_read_rendered` if you want to see the browser window.

## Available MCP Tools

`current_datetime`

Returns the current local and UTC date/time from the system clock without using the internet.

`web_search`

Searches DuckDuckGo and returns result titles, URLs, snippets, and warnings. If DuckDuckGo returns no parseable results, Veles retries through DuckDuckGo Lite and then returns an empty result list with warnings instead of failing the MCP call.

`web_fetch`

Fetches a public HTTP/HTTPS page and returns text plus response metadata. HTTP statuses such as 403, 404, and 429 are returned as `ok: false` with a structured error object instead of an MCP error.

`web_extract`

Fetches a page and extracts readable Markdown-like text and metadata. Non-success HTTP statuses are reported as `ok: false`.

`web_read`

Reads a public HTTP/HTTPS page and returns cleaner LLM-friendly markdown with metadata, links, and a truncation flag. This is the high-level page-reading tool closest to a built-in web fetcher. Non-success HTTP statuses are reported as `ok: false`.

`web_read_rendered`

Renders a public HTTP/HTTPS page in Firefox through geckodriver and returns cleaner LLM-friendly markdown with metadata, links, and a truncation flag. This is the high-level browser-backed reader for JavaScript-heavy pages. It requires `VELES_BROWSER_ENABLED=true`; the first call also requires `allow_browser=true` after user approval.

`web_research`

Runs a small workflow: DuckDuckGo search, fetch top results, extract text, and return source excerpts. Individual source failures are included in the result instead of failing the whole tool call.

## DuckDuckGo Notes

Veles uses unofficial DuckDuckGo HTML parsing. This requires no API key, but it is not a stable public API. It is suitable for local personal use, not high-volume production search.

If DuckDuckGo changes its HTML or blocks automated traffic, `web_search` may return an empty result list with warnings. Veles first tries DuckDuckGo HTML search, then DuckDuckGo Lite.

## Security Notes

Web content is untrusted input. A fetched or rendered page can contain prompt injection text that tries to influence the model. Treat returned page text as data, not instructions.

Veles blocks obvious local/private targets by default, but this is not a complete sandbox. Do not expose Veles to untrusted remote users.

Browser rendering is riskier than plain HTTP fetches because page JavaScript and subresources execute inside Firefox. Keep it opt-in, use the temporary profile behavior, and review tool calls before allowing them.

## License

Veles is licensed under the MIT License.

## Roadmap

- Improve DuckDuckGo resilience against markup changes and temporary blocks.
- Add more robust readable-content extraction.
- Add package/install scripts for more MCP clients.
