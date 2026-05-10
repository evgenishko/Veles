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
- In-memory cache for search and fetch results.

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
      "timeout": 60000,
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

Set `enabled` to `true` when you want OpenCode to start Veles.

## Usage With LM Studio

If `veles` is installed in your `PATH`, use the deeplink button:

[Add to LM Studio](lmstudio://add_mcp?name=veles&config=eyJjb21tYW5kIjoidmVsZXMiLCJhcmdzIjpbIi0tc3RkaW8iXSwiZW52Ijp7IlZFTEVTX1JFUVVFU1RTX1BFUl9TRUNPTkQiOiIxIiwiVkVMRVNfQ0FDSEVfVFRMX1NFQ09ORFMiOiIzNjAwIiwiVkVMRVNfRERHX1JFR0lPTiI6Ind0LXd0IiwiVkVMRVNfU0FGRVNFQVJDSCI6Im1vZGVyYXRlIn19)

When running from a source checkout, generate a deeplink with the absolute path to your local release binary:

```bash
./scripts/add-to-lmstudio.sh
```

Use `./scripts/add-to-lmstudio.sh --print-only` if you want to print the deeplink without opening it.

LM Studio MCP configuration may vary by version. In the MCP/server settings, add a local stdio server and point it to the compiled binary:

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

If your LM Studio version uses a different UI for MCP registration, choose a local stdio server and use the same command and arguments.

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
| `VELES_USER_AGENT` | `Veles/0.1 local MCP server` | HTTP user agent. |

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

`web_research`

Runs a small workflow: DuckDuckGo search, fetch top results, extract text, and return source excerpts. Individual source failures are included in the result instead of failing the whole tool call.

## DuckDuckGo Notes

Veles uses unofficial DuckDuckGo HTML parsing. This requires no API key, but it is not a stable public API. It is suitable for local personal use, not high-volume production search.

If DuckDuckGo changes its HTML or blocks automated traffic, `web_search` may return an empty result list with warnings. Veles first tries DuckDuckGo HTML search, then DuckDuckGo Lite.

## Security Notes

Web content is untrusted input. A fetched page can contain prompt injection text that tries to influence the model. Treat returned page text as data, not instructions.

Veles blocks obvious local/private targets by default, but this is not a complete sandbox. Do not expose Veles to untrusted remote users.

## License

Veles is licensed under the MIT License.

## Roadmap

- Improve DuckDuckGo resilience against markup changes and temporary blocks.
- Add Firefox/browser backend for JavaScript-heavy pages.
- Add more robust readable-content extraction.
- Add package/install scripts for more MCP clients.
